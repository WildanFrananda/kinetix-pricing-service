use rocket::request::{FromRequest, Outcome, Request};

use crate::error::AppError;
use crate::guards::authenticated_caller::AuthenticatedCaller;
use crate::guards::bearer::{read_bearer, refuse};
use crate::guards::bearer_failure::BearerFailure;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionalCaller(pub Option<AuthenticatedCaller>);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for OptionalCaller {
    type Error = AppError;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match read_bearer(req).await {
            Ok(claims) => return Outcome::Success(OptionalCaller(Some(claims.into()))),
            Err(BearerFailure::Absent) | Err(BearerFailure::Invalid) => {
                return Outcome::Success(OptionalCaller(None));
            }
            Err(failure) => return refuse(req, failure),
        }
    }
}
