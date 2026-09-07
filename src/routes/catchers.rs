use rocket::serde::json::Json;
use rocket::{catch, Request};

use crate::error::ErrorResponse;

#[catch(404)]
pub fn not_found(request: &Request<'_>) -> Json<ErrorResponse> {
    return Json(ErrorResponse::new(
        "NOT_FOUND",
        "no endpoint of this service serves that path.",
        request,
    ));
}

#[catch(422)]
pub fn unprocessable(request: &Request<'_>) -> Json<ErrorResponse> {
    return Json(ErrorResponse::new(
        "INVALID_BODY",
        "the request body did not match what this endpoint expects.",
        request,
    ));
}

#[catch(500)]
pub fn internal_error(request: &Request<'_>) -> Json<ErrorResponse> {
    return Json(ErrorResponse::new(
        "INTERNAL_ERROR",
        "something went wrong handling this request. No voucher, discount or quota was changed \
         unless a previous response said so.",
        request,
    ));
}

#[catch(default)]
pub fn other(status: rocket::http::Status, request: &Request<'_>) -> Json<ErrorResponse> {
    return Json(ErrorResponse::new(
        status.reason().unwrap_or("ERROR"),
        "this request was refused.",
        request,
    ));
}
