use rocket::http::Status;

pub const INVALID_REQUEST: &str = "INVALID_REQUEST";
pub const INVALID_JSON: &str = "INVALID_JSON";
pub const INVALID_BODY: &str = "INVALID_BODY";
pub const UNAUTHORIZED: &str = "UNAUTHORIZED";
pub const FORBIDDEN: &str = "FORBIDDEN";
pub const NOT_FOUND: &str = "NOT_FOUND";
pub const METHOD_NOT_ALLOWED: &str = "METHOD_NOT_ALLOWED";
pub const CONFLICT: &str = "CONFLICT";
pub const PAYLOAD_TOO_LARGE: &str = "PAYLOAD_TOO_LARGE";
pub const UNPROCESSABLE: &str = "UNPROCESSABLE";
pub const IDEMPOTENCY_KEY_REQUIRED: &str = "IDEMPOTENCY_KEY_REQUIRED";
pub const IDEMPOTENCY_KEY_TOO_LONG: &str = "IDEMPOTENCY_KEY_TOO_LONG";
pub const INTERNAL_ERROR: &str = "INTERNAL_ERROR";
pub const DATABASE_UNAVAILABLE: &str = "DATABASE_UNAVAILABLE";
pub const IDENTITY_UNAVAILABLE: &str = "IDENTITY_UNAVAILABLE";
pub const UPSTREAM_UNAVAILABLE: &str = "UPSTREAM_UNAVAILABLE";
pub const UNAVAILABLE: &str = "UNAVAILABLE";
pub const REFUSED: &str = "REFUSED";

pub fn for_status(status: Status) -> &'static str {
    match status.code {
        400 => return INVALID_REQUEST,
        401 => return UNAUTHORIZED,
        403 => return FORBIDDEN,
        404 => return NOT_FOUND,
        405 => return METHOD_NOT_ALLOWED,
        409 => return CONFLICT,
        413 => return PAYLOAD_TOO_LARGE,
        422 => return UNPROCESSABLE,
        503 => return UNAVAILABLE,
        code if code >= 500 => return INTERNAL_ERROR,
        _ => return REFUSED,
    }
}
