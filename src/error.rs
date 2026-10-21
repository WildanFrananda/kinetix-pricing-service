use diesel::result::Error;
use diesel_async::pooled_connection::deadpool::PoolError;
use rocket::http::Status;
use rocket::response::{Responder, Response, Result};
use rocket::serde::json::Json;
use rocket::Request;
use thiserror::Error;

use crate::error_code;
pub use crate::error_response::ErrorResponse;
use crate::observability::request_id::REQUEST_ID_KEY;

fn request_id<'r>(request: &'r Request<'_>) -> &'r str {
    return request.headers().get_one(REQUEST_ID_KEY).unwrap_or("-");
}

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] Error),

    #[error("Database connection pool error: {0}")]
    Pool(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    BadRequest(String),

    #[error("Unauthorized access")]
    Unauthorized,

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Unprocessable: {0}")]
    Unprocessable(String),

    #[error("Upstream unavailable: {0}")]
    Unavailable(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<PoolError> for AppError {
    fn from(err: PoolError) -> Self {
        return AppError::Pool(err.to_string());
    }
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            AppError::Database(_) | AppError::Internal(_) => return error_code::INTERNAL_ERROR,
            AppError::Pool(_) => return error_code::DATABASE_UNAVAILABLE,
            AppError::NotFound(_) => return error_code::NOT_FOUND,
            AppError::BadRequest(_) => return error_code::INVALID_REQUEST,
            AppError::Unauthorized => return error_code::UNAUTHORIZED,
            AppError::Forbidden(_) => return error_code::FORBIDDEN,
            AppError::Conflict(_) => return error_code::CONFLICT,
            AppError::Unprocessable(_) => return error_code::UNPROCESSABLE,
            AppError::Unavailable(_) => return error_code::UPSTREAM_UNAVAILABLE,
        }
    }
}

impl<'r> Responder<'r, 'static> for AppError {
    fn respond_to(self, req: &'r rocket::Request<'_>) -> Result<'static> {
        let code = self.code();
        let (status, message) = match &self {
            AppError::Database(e) => {
                tracing::error!(
                    request_id = request_id(req),
                    error = %e,
                    "a database error reached the edge"
                );
                (
                    Status::InternalServerError,
                    "something went wrong handling this request. No voucher, discount or quota \
                     was changed unless a previous response said so."
                        .to_string(),
                )
            }
            AppError::Pool(e) => {
                tracing::error!(
                    request_id = request_id(req),
                    error = %e,
                    "the connection pool could not hand out a connection"
                );
                (
                    Status::ServiceUnavailable,
                    "pricing cannot reach its database right now. Nothing was changed; try again."
                        .to_string(),
                )
            }
            AppError::NotFound(msg) => (Status::NotFound, msg.clone()),
            AppError::BadRequest(msg) => (Status::BadRequest, msg.clone()),
            AppError::Unauthorized => (Status::Unauthorized, "Unauthorized".to_string()),
            AppError::Forbidden(msg) => (Status::Forbidden, msg.clone()),
            AppError::Conflict(msg) => (Status::Conflict, msg.clone()),
            AppError::Unprocessable(msg) => (Status::UnprocessableEntity, msg.clone()),
            AppError::Unavailable(e) => {
                tracing::error!(
                    request_id = request_id(req),
                    error = %e,
                    "a dependency pricing needs was not answering"
                );
                (
                    Status::ServiceUnavailable,
                    "pricing depends on a service that is not answering right now. Nothing was \
                     changed; try again."
                        .to_string(),
                )
            }
            AppError::Internal(e) => {
                tracing::error!(
                    request_id = request_id(req),
                    error = %e,
                    "an internal error reached the edge"
                );
                (
                    Status::InternalServerError,
                    "something went wrong handling this request. No voucher, discount or quota \
                     was changed unless a previous response said so."
                        .to_string(),
                )
            }
        };

        let err_json = Json(ErrorResponse::new(code, &message, req));

        return Response::build_from(err_json.respond_to(req)?)
            .status(status)
            .ok();
    }
}
