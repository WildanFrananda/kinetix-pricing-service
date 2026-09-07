use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{Duration, Instant};

use crate::security::jwks_breaker::JwksBreaker;

const JWKS_COOLDOWN: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, Deserialize)]
pub struct AccessClaims {
    pub sub: String,
    pub jti: String,
    pub exp: usize,
    pub token_use: String,
    pub uid: i64,
    pub email: String,
    pub role: String,
}

#[derive(Debug, Deserialize)]
struct Jwk {
    kid: String,
    n: String,
    e: String,
    #[serde(default)]
    alg: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Jwks {
    keys: Vec<Jwk>,
}

pub struct JwtVerifier {
    jwks_url: String,
    issuer: String,
    audience: String,
    keys: RwLock<HashMap<String, DecodingKey>>,
    http: reqwest::Client,
    breaker: JwksBreaker,
    refresh_lock: tokio::sync::Mutex<()>,
}

#[derive(Debug)]
pub enum JwtError {
    Missing,
    Invalid,
    Unavailable(String),
}

impl JwtVerifier {
    pub fn from_env() -> Result<Self, String> {
        let jwks_url = std::env::var("IDENTITY_JWKS_URL")
            .map_err(|_| "IDENTITY_JWKS_URL is required and has no default".to_string())?;
        let issuer = std::env::var("JWT_ISSUER")
            .map_err(|_| "JWT_ISSUER is required and has no default".to_string())?;
        let audience = std::env::var("JWT_AUDIENCE")
            .map_err(|_| "JWT_AUDIENCE is required and has no default".to_string())?;

        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(2))
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| format!("cannot build the JWKS HTTP client: {e}"))?;

        return Ok(Self {
            jwks_url,
            issuer,
            audience,
            keys: RwLock::new(HashMap::new()),
            http,
            breaker: JwksBreaker::new(JWKS_COOLDOWN),
            refresh_lock: tokio::sync::Mutex::new(()),
        });
    }

    pub async fn refresh(&self) -> Result<usize, String> {
        if let Some(remaining) = self.breaker.cooldown_remaining() {
            return Err(self.cooldown_message(remaining));
        }

        let queued_at = Instant::now();
        let _flight = self.refresh_lock.lock().await;

        if self.breaker.refreshed_since(queued_at) {
            return self.key_count();
        }
        if let Some(remaining) = self.breaker.cooldown_remaining() {
            return Err(self.cooldown_message(remaining));
        }

        match self.fetch_keys().await {
            Ok(count) => {
                self.breaker.record_success();
                return Ok(count);
            }
            Err(e) => {
                self.breaker.record_failure();
                return Err(e);
            }
        }
    }

    async fn fetch_keys(&self) -> Result<usize, String> {
        let body: Jwks = self
            .http
            .get(&self.jwks_url)
            .send()
            .await
            .map_err(|e| format!("cannot reach {}: {e}", self.jwks_url))?
            .json()
            .await
            .map_err(|e| format!("{} did not return a JWKS: {e}", self.jwks_url))?;

        let mut fresh = HashMap::new();
        for jwk in body.keys {
            if jwk.alg.as_deref().unwrap_or("RS256") != "RS256" {
                continue;
            }
            let key = DecodingKey::from_rsa_components(&jwk.n, &jwk.e)
                .map_err(|e| format!("JWKS key {} is unusable: {e}", jwk.kid))?;
            fresh.insert(jwk.kid, key);
        }

        if fresh.is_empty() {
            return Err(format!("{} published no usable RS256 keys", self.jwks_url));
        }

        let count = fresh.len();
        *self
            .keys
            .write()
            .map_err(|_| "the key cache is poisoned".to_string())? = fresh;
        return Ok(count);
    }

    fn key_count(&self) -> Result<usize, String> {
        let guard = self
            .keys
            .read()
            .map_err(|_| "the key cache is poisoned".to_string())?;
        return Ok(guard.len());
    }

    fn cooldown_message(&self, remaining: Duration) -> String {
        return format!(
            "{} failed recently and is not being called again for {:.1}s",
            self.jwks_url,
            remaining.as_secs_f32()
        );
    }

    pub async fn verify_access(&self, token: &str) -> Result<AccessClaims, JwtError> {
        let header = decode_header(token).map_err(|_| JwtError::Invalid)?;

        if header.alg != Algorithm::RS256 {
            return Err(JwtError::Invalid);
        }
        let kid = header.kid.ok_or(JwtError::Invalid)?;

        let mut key = self.lookup(&kid)?;
        if key.is_none() {
            self.refresh().await.map_err(JwtError::Unavailable)?;
            key = self.lookup(&kid)?;
        }
        let key = key.ok_or(JwtError::Invalid)?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[self.issuer.as_str()]);
        validation.set_audience(&[self.audience.as_str()]);
        validation.validate_exp = true;

        let data =
            decode::<AccessClaims>(token, &key, &validation).map_err(|_| JwtError::Invalid)?;

        if data.claims.token_use != "access" {
            return Err(JwtError::Invalid);
        }

        return Ok(data.claims);
    }

    fn lookup(&self, kid: &str) -> Result<Option<DecodingKey>, JwtError> {
        let guard = self
            .keys
            .read()
            .map_err(|_| JwtError::Unavailable("the key cache is poisoned".to_string()))?;
        return Ok(guard.get(kid).cloned());
    }
}
