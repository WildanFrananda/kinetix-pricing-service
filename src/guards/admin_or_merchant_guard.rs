use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};

use crate::error::AppError;
use crate::error_code;
use crate::guards::bearer::{read_bearer, refuse};
use crate::guards::guard_refusal::GuardRefusal;

const ADMIN_OR_SELLER: &str = "this endpoint needs the admin or seller role.";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdminOrMerchantGuard {
    pub principal_id: String,
    pub role: String,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AdminOrMerchantGuard {
    type Error = AppError;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let claims = match read_bearer(req).await {
            Ok(claims) => claims,
            Err(failure) => return refuse(req, failure),
        };

        if claims.role != "admin" && claims.role != "seller" {
            GuardRefusal::record(req, error_code::FORBIDDEN, ADMIN_OR_SELLER);
            return Outcome::Error((
                Status::Forbidden,
                AppError::Forbidden(ADMIN_OR_SELLER.to_string()),
            ));
        }

        return Outcome::Success(AdminOrMerchantGuard {
            principal_id: claims.sub,
            role: claims.role,
        });
    }
}
