use rocket::http::Status;
use rocket::response::{self, Responder, Response};
use rocket::serde::json::Json;
use serde::Serialize;
use thiserror::Error;

use crate::observability::request_id::REQUEST_ID_KEY;

fn request_id<'r>(request: &'r rocket::Request<'_>) -> &'r str {
    return request.headers().get_one(REQUEST_ID_KEY).unwrap_or("-");
}

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] diesel::result::Error),

    #[error("Database connection pool error: {0}")]
    Pool(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    BadRequest(String),

    #[error("Unauthorized access")]
    Unauthorized,

    #[error("Upstream unavailable: {0}")]
    Unavailable(String),
}

impl From<diesel_async::pooled_connection::deadpool::PoolError> for AppError {
    fn from(err: diesel_async::pooled_connection::deadpool::PoolError) -> Self {
        return AppError::Pool(err.to_string());
    }
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    #[serde(rename = "traceId")]
    pub trace_id: String,
}

impl ErrorResponse {
    pub fn new(error: &str, message: &str, request: &rocket::Request<'_>) -> Self {
        return Self {
            error: error.to_string(),
            message: message.to_string(),
            trace_id: request
                .headers()
                .get_one(REQUEST_ID_KEY)
                .unwrap_or("-")
                .to_string(),
        };
    }
}

impl<'r> Responder<'r, 'static> for AppError {
    fn respond_to(self, req: &'r rocket::Request<'_>) -> response::Result<'static> {
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
                    Status::InternalServerError,
                    "pricing is not able to answer right now. Nothing was changed; try again."
                        .to_string(),
                )
            }
            AppError::NotFound(msg) => (Status::NotFound, msg.clone()),
            AppError::BadRequest(msg) => (Status::BadRequest, msg.clone()),
            AppError::Unauthorized => (Status::Unauthorized, "Unauthorized".to_string()),
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
        };

        let err_json = Json(ErrorResponse::new(
            status.reason().unwrap_or("Error"),
            &message,
            req,
        ));

        return Response::build_from(err_json.respond_to(req)?)
            .status(status)
            .ok();
    }
}
