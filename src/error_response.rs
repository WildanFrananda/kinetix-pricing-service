use rocket::Request;
use serde::Serialize;

use crate::observability::request_id::REQUEST_ID_KEY;

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    #[serde(rename = "traceId")]
    pub trace_id: String,
}

impl ErrorResponse {
    pub fn new(error: &str, message: &str, request: &Request<'_>) -> Self {
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
