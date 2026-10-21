use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{catch, Request};

use crate::error::ErrorResponse;
use crate::error_code;
use crate::guards::GuardRefusal;
use crate::routes::allowed_methods::allowed_methods;
use crate::routes::method_not_allowed::MethodNotAllowed;

#[catch(404)]
pub fn not_found(request: &Request<'_>) -> Result<Json<ErrorResponse>, MethodNotAllowed> {
    let allowed = allowed_methods(request);
    if !allowed.is_empty() && !allowed.contains(&request.method()) {
        return Err(MethodNotAllowed::new(request, &allowed));
    }

    return Ok(Json(ErrorResponse::new(
        error_code::NOT_FOUND,
        "no endpoint of this service serves that path.",
        request,
    )));
}

#[catch(422)]
pub fn unprocessable(request: &Request<'_>) -> Json<ErrorResponse> {
    if let Some(refusal) = GuardRefusal::of(request) {
        return Json(ErrorResponse::new(refusal.error, &refusal.message, request));
    }

    return Json(ErrorResponse::new(
        error_code::INVALID_BODY,
        "the request body did not match what this endpoint expects.",
        request,
    ));
}

#[catch(500)]
pub fn internal_error(request: &Request<'_>) -> Json<ErrorResponse> {
    return Json(ErrorResponse::new(
        error_code::INTERNAL_ERROR,
        "something went wrong handling this request. No voucher, discount or quota was changed \
         unless a previous response said so.",
        request,
    ));
}

#[catch(default)]
pub fn other(status: Status, request: &Request<'_>) -> Json<ErrorResponse> {
    if let Some(refusal) = GuardRefusal::of(request) {
        return Json(ErrorResponse::new(refusal.error, &refusal.message, request));
    }

    return Json(ErrorResponse::new(
        error_code::for_status(status),
        "this request was refused.",
        request,
    ));
}
