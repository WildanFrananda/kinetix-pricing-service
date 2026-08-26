use rocket::http::Status;
use rocket::response::{self, Responder, Response};
use rocket::serde::json::Json;
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    BadRequest(String),

    #[error("Unauthorized access")]
    Unauthorized,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}

impl<'r> Responder<'r, 'static> for AppError {
    fn respond_to(self, req: &'r rocket::Request<'_>) -> response::Result<'static> {
        let (status, message) = match &self {
            AppError::Database(e) => (Status::InternalServerError, format!("Database error: {}", e)),
            AppError::NotFound(msg) => (Status::NotFound, msg.clone()),
            AppError::BadRequest(msg) => (Status::BadRequest, msg.clone()),
            AppError::Unauthorized => (Status::Unauthorized, "Unauthorized".to_string()),
        };

        let err_json = Json(ErrorResponse {
            error: status.reason().unwrap_or("Error").to_string(),
            message,
        });

        return Response::build_from(err_json.respond_to(req)?)
            .status(status)
            .ok();
    }
}
