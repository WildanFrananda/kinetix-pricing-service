use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};

use crate::error::AppError;
use crate::error_code;
use crate::guards::guard_refusal::GuardRefusal;

pub const IDEMPOTENCY_KEY_HEADER: &str = "Idempotency-Key";
pub const IDEMPOTENCY_KEY_MAX_LENGTH: usize = 255;

const REQUIRED: &str =
    "a create needs an Idempotency-Key header naming this one promotion. A retry \
                        sends the same key and gets back what the first attempt created.";
const TOO_LONG: &str = "an Idempotency-Key is at most 255 characters.";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdempotencyKey(pub String);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for IdempotencyKey {
    type Error = AppError;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let key = req
            .headers()
            .get_one(IDEMPOTENCY_KEY_HEADER)
            .map(str::trim)
            .unwrap_or("");

        if key.is_empty() {
            GuardRefusal::record(req, error_code::IDEMPOTENCY_KEY_REQUIRED, REQUIRED);
            return Outcome::Error((
                Status::BadRequest,
                AppError::BadRequest(REQUIRED.to_string()),
            ));
        }
        if key.chars().count() > IDEMPOTENCY_KEY_MAX_LENGTH {
            GuardRefusal::record(req, error_code::IDEMPOTENCY_KEY_TOO_LONG, TOO_LONG);
            return Outcome::Error((
                Status::BadRequest,
                AppError::BadRequest(TOO_LONG.to_string()),
            ));
        }

        return Outcome::Success(IdempotencyKey(key.to_string()));
    }
}
