use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};

use crate::error::AppError;
use crate::security::jwt::{AccessClaims, JwtVerifier};

pub struct AuthenticatedCaller {
    pub principal_id: String,
    pub user_id: i64,
    pub email: String,
    pub role: String,
}

impl From<AccessClaims> for AuthenticatedCaller {
    fn from(c: AccessClaims) -> Self {
        Self { principal_id: c.sub, user_id: c.uid, email: c.email, role: c.role }
    }
}

async fn verify_bearer(req: &Request<'_>) -> Result<AccessClaims, (Status, AppError)> {
    let verifier = match req.rocket().state::<JwtVerifier>() {
        Some(v) => v,
        None => {
            return Err((
                Status::InternalServerError,
                AppError::BadRequest("the token verifier is not configured".to_string()),
            ))
        }
    };

    let header = req.headers().get_one("Authorization").unwrap_or("");
    let mut parts = header.split(' ');
    let scheme = parts.next().unwrap_or("");
    let token = parts.next().unwrap_or("");

    if !scheme.eq_ignore_ascii_case("bearer") || token.is_empty() || parts.next().is_some() {
        return Err((Status::Unauthorized, AppError::Unauthorized));
    }

    verifier
        .verify_access(token)
        .await
        .map_err(|_| (Status::Unauthorized, AppError::Unauthorized))
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthenticatedCaller {
    type Error = AppError;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match verify_bearer(req).await {
            Ok(claims) => Outcome::Success(claims.into()),
            Err((status, err)) => Outcome::Error((status, err)),
        }
    }
}

pub struct AdminOrMerchantGuard {
    pub principal_id: String,
    pub user_id: i64,
    pub role: String,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AdminOrMerchantGuard {
    type Error = AppError;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let claims = match verify_bearer(req).await {
            Ok(c) => c,
            Err((status, err)) => return Outcome::Error((status, err)),
        };

        if claims.role != "admin" && claims.role != "seller" {
            return Outcome::Error((
                Status::Forbidden,
                AppError::BadRequest("Forbidden: requires the admin or seller role".to_string()),
            ));
        }

        Outcome::Success(AdminOrMerchantGuard {
            principal_id: claims.sub,
            user_id: claims.uid,
            role: claims.role,
        })
    }
}
