use rocket::http::Status;
use rocket::request::{Outcome, Request};

use crate::error::AppError;
use crate::error_code;
use crate::guards::bearer_failure::BearerFailure;
use crate::guards::guard_refusal::GuardRefusal;
use crate::observability::request_id::REQUEST_ID_KEY;
use crate::observability::AdminActor;
use crate::security::jwt::{AccessClaims, JwtError, JwtVerifier};

const ADMIN_ROLE: &str = "admin";
const UNAUTHORIZED: &str = "this endpoint needs a valid identity access token.";
const IDENTITY_UNAVAILABLE: &str = "pricing cannot check the access token because identity's \
                                    signing keys cannot be fetched right now. Nothing was \
                                    changed; try again.";

pub async fn read_bearer(req: &Request<'_>) -> Result<AccessClaims, BearerFailure> {
    let Some(header) = req.headers().get_one("Authorization") else {
        return Err(BearerFailure::Absent);
    };

    let mut parts = header.split(' ');
    let scheme = parts.next().unwrap_or("");
    let token = parts.next().unwrap_or("");

    if !scheme.eq_ignore_ascii_case("bearer") || token.is_empty() || parts.next().is_some() {
        return Err(BearerFailure::Invalid);
    }

    let Some(verifier) = req.rocket().state::<JwtVerifier>() else {
        return Err(BearerFailure::Unconfigured);
    };

    match verifier.verify_access(token).await {
        Ok(claims) => {
            if claims.role == ADMIN_ROLE {
                AdminActor::record(req, &claims.sub);
            }
            return Ok(claims);
        }
        Err(JwtError::Unavailable(reason)) => {
            tracing::error!(
                request_id = req.headers().get_one(REQUEST_ID_KEY).unwrap_or("-"),
                error = %reason,
                "cannot verify a token because identity's JWKS is unreachable"
            );
            return Err(BearerFailure::Unavailable(reason));
        }
        Err(JwtError::Missing) | Err(JwtError::Invalid) => return Err(BearerFailure::Invalid),
    }
}

pub fn refuse<T>(req: &Request<'_>, failure: BearerFailure) -> Outcome<T, AppError> {
    match failure {
        BearerFailure::Absent | BearerFailure::Invalid => {
            GuardRefusal::record(req, error_code::UNAUTHORIZED, UNAUTHORIZED);
            return Outcome::Error((Status::Unauthorized, AppError::Unauthorized));
        }
        BearerFailure::Unavailable(reason) => {
            GuardRefusal::record(req, error_code::IDENTITY_UNAVAILABLE, IDENTITY_UNAVAILABLE);
            return Outcome::Error((Status::ServiceUnavailable, AppError::Unavailable(reason)));
        }
        BearerFailure::Unconfigured => {
            tracing::error!(
                request_id = req.headers().get_one(REQUEST_ID_KEY).unwrap_or("-"),
                "a route needs the token verifier, and none is managed"
            );
            return Outcome::Error((
                Status::InternalServerError,
                AppError::Internal("the token verifier is not configured".to_string()),
            ));
        }
    }
}
