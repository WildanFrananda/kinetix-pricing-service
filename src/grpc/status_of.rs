use tonic::Status;

use crate::error::AppError;

pub fn status_of(error: AppError, rpc: &str) -> Status {
    match error {
        AppError::BadRequest(message) | AppError::Unprocessable(message) => {
            return Status::invalid_argument(format!("{rpc}: {message}"));
        }
        AppError::NotFound(message) => return Status::not_found(format!("{rpc}: {message}")),
        AppError::Pool(reason) | AppError::Unavailable(reason) => {
            tracing::error!(rpc, error = %reason, "a dependency pricing needs was not answering");
            return Status::unavailable(format!("{rpc}: pricing cannot answer right now"));
        }
        other => {
            tracing::error!(rpc, error = %other, "an RPC failed inside pricing");
            return Status::internal(format!("{rpc} failed inside pricing"));
        }
    }
}
