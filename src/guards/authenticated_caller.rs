use rocket::request::{FromRequest, Outcome, Request};

use crate::error::AppError;
use crate::guards::bearer::{read_bearer, refuse};
use crate::security::jwt::AccessClaims;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedCaller {
    pub principal_id: String,
    pub email: String,
    pub role: String,
}

impl From<AccessClaims> for AuthenticatedCaller {
    fn from(c: AccessClaims) -> Self {
        return Self {
            principal_id: c.sub,
            email: c.email,
            role: c.role,
        };
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthenticatedCaller {
    type Error = AppError;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match read_bearer(req).await {
            Ok(claims) => return Outcome::Success(claims.into()),
            Err(failure) => return refuse(req, failure),
        }
    }
}
