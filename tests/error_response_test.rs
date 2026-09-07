use rocket::http::{Header, Status};
use rocket::local::blocking::Client;
use rocket::serde::json::Json;
use rocket::{catchers, get, routes};
use serde_json::Value;

use kinetix_pricing_service::error::{AppError, ErrorResponse};
use kinetix_pricing_service::routes::catchers::{internal_error, not_found, other, unprocessable};

const ID: &str = "kinetix-trace-under-test";

#[get("/boom/database")]
fn boom_database() -> Result<&'static str, AppError> {
    return Err(AppError::Database(diesel::result::Error::NotFound));
}

#[get("/boom/pool")]
fn boom_pool() -> Result<&'static str, AppError> {
    return Err(AppError::Pool(
        "connection refused talking to 10.77.0.5:5432 as kinetix_pricing_app".to_string(),
    ));
}

#[get("/boom/missing")]
fn boom_missing() -> Result<&'static str, AppError> {
    return Err(AppError::NotFound(
        "voucher PROMO50 does not exist".to_string(),
    ));
}

#[get("/boom/unauthorized")]
fn boom_unauthorized() -> Result<&'static str, AppError> {
    return Err(AppError::Unauthorized);
}

#[get("/echo")]
fn echo() -> Json<&'static str> {
    return Json("ok");
}

fn client() -> Client {
    let rocket = rocket::build()
        .mount(
            "/",
            routes![
                boom_database,
                boom_pool,
                boom_missing,
                boom_unauthorized,
                echo
            ],
        )
        .register(
            "/",
            catchers![not_found, unprocessable, internal_error, other],
        );

    return Client::tracked(rocket).expect("the test rocket did not build");
}

fn body_of(response: rocket::local::blocking::LocalResponse<'_>) -> Value {
    let raw = response
        .into_string()
        .expect("the error response had no body");
    return serde_json::from_str(&raw).expect("the error response was not JSON");
}

#[test]
fn an_unrouted_path_is_answered_by_the_404_catcher_with_the_callers_id() {
    let client = client();
    let response = client
        .get("/no/such/endpoint")
        .header(Header::new("X-Request-Id", ID))
        .dispatch();

    assert_eq!(response.status(), Status::NotFound);

    let body = body_of(response);
    assert_eq!(body["error"], "NOT_FOUND");
    assert_eq!(body["traceId"], ID);
}

#[test]
fn a_caller_that_sends_no_id_gets_a_dash_rather_than_an_invented_one() {
    let client = client();
    let response = client.get("/no/such/endpoint").dispatch();

    assert_eq!(response.status(), Status::NotFound);
    assert_eq!(body_of(response)["traceId"], "-");
}

#[test]
fn a_database_error_reaches_the_caller_as_a_sentence_and_not_as_diesel() {
    let client = client();
    let response = client
        .get("/boom/database")
        .header(Header::new("X-Request-Id", ID))
        .dispatch();

    assert_eq!(response.status(), Status::InternalServerError);

    let body = body_of(response);
    assert_eq!(body["traceId"], ID);

    let message = body["message"].as_str().expect("no message in the body");
    assert!(
        message.contains("No voucher, discount or quota was changed"),
        "the body should say what was not changed, got: {message}"
    );
    assert!(
        !message.contains("Record not found"),
        "the driver's error text reached the caller: {message}"
    );
    assert!(
        !message.to_lowercase().contains("diesel"),
        "the driver was named to the caller: {message}"
    );
}

#[test]
fn a_pool_error_never_names_the_host_it_could_not_reach() {
    let client = client();
    let response = client
        .get("/boom/pool")
        .header(Header::new("X-Request-Id", ID))
        .dispatch();

    assert_eq!(response.status(), Status::InternalServerError);

    let body = body_of(response);
    assert_eq!(body["traceId"], ID);

    let message = body["message"].as_str().expect("no message in the body");
    assert!(
        !message.contains("10.77.0.5"),
        "an internal address reached the caller: {message}"
    );
    assert!(
        !message.contains("kinetix_pricing_app"),
        "a database role name reached the caller: {message}"
    );
}

#[test]
fn a_not_found_keeps_the_message_the_service_wrote() {
    let client = client();
    let response = client
        .get("/boom/missing")
        .header(Header::new("X-Request-Id", ID))
        .dispatch();

    assert_eq!(response.status(), Status::NotFound);

    let body = body_of(response);
    assert_eq!(body["message"], "voucher PROMO50 does not exist");
    assert_eq!(body["traceId"], ID);
}

#[test]
fn an_unauthorized_call_is_401_and_still_carries_the_id() {
    let client = client();
    let response = client
        .get("/boom/unauthorized")
        .header(Header::new("X-Request-Id", ID))
        .dispatch();

    assert_eq!(response.status(), Status::Unauthorized);
    assert_eq!(body_of(response)["traceId"], ID);
}

#[test]
fn the_default_catcher_answers_a_status_no_other_catcher_claims() {
    let rocket = rocket::build()
        .mount("/", routes![echo])
        .register("/", catchers![other]);
    let client = Client::tracked(rocket).expect("the test rocket did not build");

    let response = client
        .get("/no/such/endpoint")
        .header(Header::new("X-Request-Id", ID))
        .dispatch();

    assert_eq!(response.status(), Status::NotFound);

    let body = body_of(response);
    assert_eq!(body["error"], "Not Found");
    assert_eq!(body["traceId"], ID);
}

#[test]
fn error_response_reads_the_id_off_the_request_it_was_built_for() {
    let client = client();
    let response = client
        .get("/no/such/endpoint")
        .header(Header::new("x-request-id", "lowercase-header-still-works"))
        .dispatch();

    assert_eq!(body_of(response)["traceId"], "lowercase-header-still-works");

    let _ = std::mem::size_of::<ErrorResponse>();
}
