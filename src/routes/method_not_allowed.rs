use rocket::http::{Header, Method};
use rocket::serde::json::Json;
use rocket::{Request, Responder};

use crate::error::ErrorResponse;
use crate::error_code;

#[derive(Responder)]
#[response(status = 405)]
pub struct MethodNotAllowed {
    body: Json<ErrorResponse>,
    allow: Header<'static>,
}

impl MethodNotAllowed {
    pub fn new(request: &Request<'_>, allowed: &[Method]) -> Self {
        let methods = allowed
            .iter()
            .map(|method| return method.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let message = format!(
            "this path does not take {}; it takes {methods}.",
            request.method()
        );

        return Self {
            body: Json(ErrorResponse::new(
                error_code::METHOD_NOT_ALLOWED,
                &message,
                request,
            )),
            allow: Header::new("Allow", methods),
        };
    }
}
