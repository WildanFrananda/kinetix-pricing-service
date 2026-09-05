use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::RwLock;

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

        Ok(Self { jwks_url, issuer, audience, keys: RwLock::new(HashMap::new()) })
    }

    pub async fn refresh(&self) -> Result<usize, String> {
        let body: Jwks = reqwest::get(&self.jwks_url)
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
        *self.keys.write().map_err(|_| "the key cache is poisoned".to_string())? = fresh;
        Ok(count)
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

        let data = decode::<AccessClaims>(token, &key, &validation).map_err(|_| JwtError::Invalid)?;

        if data.claims.token_use != "access" {
            return Err(JwtError::Invalid);
        }

        Ok(data.claims)
    }

    fn lookup(&self, kid: &str) -> Result<Option<DecodingKey>, JwtError> {
        let guard = self
            .keys
            .read()
            .map_err(|_| JwtError::Unavailable("the key cache is poisoned".to_string()))?;
        Ok(guard.get(kid).cloned())
    }
}
