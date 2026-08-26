use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};

use crate::error::AppError;

pub struct AdminOrMerchantGuard {
    pub role: String,
    pub user_id: Option<String>,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AdminOrMerchantGuard {
    type Error = AppError;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let role_header = req.headers().get_one("X-User-Role");
        let user_id_header = req.headers().get_one("X-User-Id").map(|s| {
            return s.to_string();
        });

        match role_header {
            Some(role) if role == "ADMIN" || role == "MERCHANT" => {
                return Outcome::Success(AdminOrMerchantGuard {
                    role: role.to_string(),
                    user_id: user_id_header,
                });
            }
            Some(_) => {
                return Outcome::Error((
                    Status::Forbidden,
                    AppError::BadRequest("Forbidden: Require ADMIN or MERCHANT role".to_string()),
                ));
            }
            None => {
                return Outcome::Error((
                    Status::Unauthorized,
                    AppError::Unauthorized,
                ));
            }
        }
    }
}
