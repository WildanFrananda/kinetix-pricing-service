use std::sync::{Arc, Once};

use async_trait::async_trait;
use rocket::http::{Header, Status};
use rocket::local::blocking::{Client, LocalResponse};
use rocket::serde::json::Json;
use rocket::{catchers, get, post, routes};
use serde_json::Value;

use kinetix_pricing_service::clients::{
    MerchantDirectoryPort, MerchantStanding, PeerError, ProductDirectoryPort,
};
use kinetix_pricing_service::db::create_pool;
use kinetix_pricing_service::guards::{IdempotencyKey, JsonBody, OptionalCaller};
use kinetix_pricing_service::models::CreateVoucherRequest;
use kinetix_pricing_service::routes::catchers::{internal_error, not_found, other, unprocessable};
use kinetix_pricing_service::routes::created_response::REPLAYED_HEADER;
use kinetix_pricing_service::routes::discount_routes::{create_discount, list_discounts};
use kinetix_pricing_service::routes::flash_sale_routes::{
    create_flash_sale, get_flash_sale_for_product,
};
use kinetix_pricing_service::routes::health_routes::health_check;
use kinetix_pricing_service::routes::voucher_routes::{apply_voucher, create_voucher, get_voucher};
use kinetix_pricing_service::routes::CreatedResponse;
use kinetix_pricing_service::security::jwt::JwtVerifier;
use kinetix_pricing_service::services::PromotionService;

const FORGED: &str = "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCIsImtpZCI6Im5vdC1wdWJsaXNoZWQifQ.\
                      eyJzdWIiOiJwIn0.c2lnbmF0dXJl";

struct UnreachableIdentity;
struct UnreachableCatalog;

#[async_trait]
impl MerchantDirectoryPort for UnreachableIdentity {
    async fn standing_of(
        &self,
        _principal_id: &str,
    ) -> Result<Option<MerchantStanding>, PeerError> {
        return Err(PeerError::Unavailable(
            "identity did not answer".to_string(),
        ));
    }
}

#[async_trait]
impl ProductDirectoryPort for UnreachableCatalog {
    async fn owner_of(&self, _sku: &str) -> Result<Option<String>, PeerError> {
        return Err(PeerError::Unavailable("catalog did not answer".to_string()));
    }
}

#[post("/stub/created?<replayed>")]
fn stub_created(replayed: bool) -> CreatedResponse<Json<&'static str>> {
    return CreatedResponse::new(Json("made"), replayed);
}

#[post("/stub/keyed")]
fn stub_keyed(key: IdempotencyKey) -> String {
    return key.0;
}

#[post("/stub/body", data = "<body>")]
fn stub_body(body: JsonBody<CreateVoucherRequest>) -> String {
    return body.into_inner().code;
}

#[get("/stub/viewer")]
fn stub_viewer(viewer: OptionalCaller) -> String {
    return format!("{}", viewer.0.is_some());
}

static IDENTITY_IS_DOWN: Once = Once::new();

fn client() -> Client {
    IDENTITY_IS_DOWN.call_once(|| {
        std::env::set_var(
            "IDENTITY_JWKS_URL",
            "http://127.0.0.1:1/.well-known/jwks.json",
        );
        std::env::set_var("JWT_ISSUER", "https://identity.test");
        std::env::set_var("JWT_AUDIENCE", "kinetix");
    });

    let rocket = rocket::build()
        .manage(create_pool("postgres://unused:unused@127.0.0.1:1/unused"))
        .manage(JwtVerifier::from_env().expect("the verifier reads its three variables"))
        .manage(PromotionService::new(
            Arc::new(UnreachableIdentity),
            Arc::new(UnreachableCatalog),
        ))
        .mount(
            "/",
            routes![
                health_check,
                create_discount,
                list_discounts,
                create_voucher,
                apply_voucher,
                get_voucher,
                create_flash_sale,
                get_flash_sale_for_product,
                stub_created,
                stub_keyed,
                stub_body,
                stub_viewer
            ],
        )
        .register(
            "/",
            catchers![not_found, unprocessable, internal_error, other],
        );

    return Client::tracked(rocket).expect("the test rocket did not build");
}

fn body_of(response: LocalResponse<'_>) -> Value {
    let raw = response.into_string().expect("the response had no body");
    return serde_json::from_str(&raw).expect("the response was not JSON");
}

#[test]
fn a_create_with_no_token_is_401_and_says_a_token_is_needed() {
    let client = client();
    let response = client.post("/api/v1/vouchers").body("{}").dispatch();

    assert_eq!(response.status(), Status::Unauthorized);

    let body = body_of(response);
    assert_eq!(body["error"], "UNAUTHORIZED");
    assert!(
        body["message"]
            .as_str()
            .unwrap_or("")
            .contains("access token"),
        "{body}"
    );
}

#[test]
fn a_create_while_identitys_keys_cannot_be_fetched_is_503_and_says_so() {
    let client = client();

    for path in [
        "/api/v1/vouchers",
        "/api/v1/discounts",
        "/api/v1/flash-sales",
    ] {
        let response = client
            .post(path)
            .header(Header::new("Authorization", format!("Bearer {FORGED}")))
            .header(Header::new("Idempotency-Key", "k-1"))
            .body("{}")
            .dispatch();

        assert_eq!(response.status(), Status::ServiceUnavailable, "{path}");

        let body = body_of(response);
        assert_eq!(body["error"], "IDENTITY_UNAVAILABLE", "{path}: {body}");
        assert!(
            body["message"].as_str().unwrap_or("").contains("identity"),
            "{path}: {body}"
        );
    }
}

#[test]
fn a_public_read_with_a_token_identity_cannot_check_is_503_not_a_quiet_public_answer() {
    let client = client();
    let response = client
        .get("/stub/viewer")
        .header(Header::new("Authorization", format!("Bearer {FORGED}")))
        .dispatch();

    assert_eq!(response.status(), Status::ServiceUnavailable);
    assert_eq!(body_of(response)["error"], "IDENTITY_UNAVAILABLE");
}

#[test]
fn a_public_read_with_no_token_or_a_malformed_one_is_read_anonymously() {
    let client = client();

    let anonymous = client.get("/stub/viewer").dispatch();
    assert_eq!(anonymous.status(), Status::Ok);
    assert_eq!(anonymous.into_string().as_deref(), Some("false"));

    let malformed = client
        .get("/stub/viewer")
        .header(Header::new("Authorization", "Basic dXNlcjpwYXNz"))
        .dispatch();
    assert_eq!(malformed.status(), Status::Ok);
    assert_eq!(malformed.into_string().as_deref(), Some("false"));
}

#[test]
fn a_create_without_an_idempotency_key_is_refused_with_the_reason() {
    let client = client();

    for header in [None, Some(""), Some("   ")] {
        let mut request = client.post("/stub/keyed");
        if let Some(value) = header {
            request = request.header(Header::new("Idempotency-Key", value));
        }
        let response = request.dispatch();

        assert_eq!(response.status(), Status::BadRequest, "{header:?}");
        assert_eq!(
            body_of(response)["error"],
            "IDEMPOTENCY_KEY_REQUIRED",
            "{header:?}"
        );
    }
}

#[test]
fn an_idempotency_key_longer_than_its_column_is_refused() {
    let client = client();
    let response = client
        .post("/stub/keyed")
        .header(Header::new("Idempotency-Key", "k".repeat(256)))
        .dispatch();

    assert_eq!(response.status(), Status::BadRequest);
    assert_eq!(body_of(response)["error"], "IDEMPOTENCY_KEY_TOO_LONG");

    let fits = client
        .post("/stub/keyed")
        .header(Header::new("Idempotency-Key", "k".repeat(255)))
        .dispatch();
    assert_eq!(fits.status(), Status::Ok);
}

#[test]
fn a_create_answers_201_and_a_replay_says_it_is_one() {
    let client = client();

    let first = client.post("/stub/created?replayed=false").dispatch();
    assert_eq!(first.status(), Status::Created);
    assert!(first.headers().get_one(REPLAYED_HEADER).is_none());

    let again = client.post("/stub/created?replayed=true").dispatch();
    assert_eq!(again.status(), Status::Created);
    assert_eq!(again.headers().get_one(REPLAYED_HEADER), Some("true"));
}

#[test]
fn a_method_a_path_does_not_take_is_405_with_the_methods_it_does() {
    let client = client();
    let cases = [
        ("/api/v1/vouchers", "POST"),
        ("/api/v1/discounts", "GET, HEAD, POST"),
        ("/api/v1/flash-sales/SKU-1", "GET, HEAD"),
        ("/health", "GET, HEAD"),
    ];

    for (path, allowed) in cases {
        let response = client.delete(path).dispatch();

        assert_eq!(response.status(), Status::MethodNotAllowed, "{path}");
        assert_eq!(response.headers().get_one("Allow"), Some(allowed), "{path}");
        assert_eq!(body_of(response)["error"], "METHOD_NOT_ALLOWED", "{path}");
    }
}

#[test]
fn a_put_on_a_create_path_is_405_and_a_get_on_it_is_too() {
    let client = client();

    assert_eq!(
        client.put("/api/v1/flash-sales").dispatch().status(),
        Status::MethodNotAllowed
    );
    assert_eq!(
        client.get("/api/v1/vouchers").dispatch().status(),
        Status::MethodNotAllowed
    );
}

#[test]
fn a_path_no_route_serves_is_still_404() {
    let client = client();

    for path in [
        "/no/such/endpoint",
        "/api/v1/vouchers/a/b",
        "/api/v2/discounts",
    ] {
        let response = client.delete(path).dispatch();
        assert_eq!(response.status(), Status::NotFound, "{path}");
        assert_eq!(body_of(response)["error"], "NOT_FOUND", "{path}");
    }
}

const GOOD_VOUCHER: &str = r#"{"code":"SAVE10","title":"Ten off","discount_type":"FIXED","value":"10000","min_spend":"0","max_discount":null,"quota":5,"expires_at":"2099-01-01T00:00:00Z"}"#;

fn body_answer(client: &Client, body: &str) -> (Status, Value) {
    let response = client.post("/stub/body").body(body).dispatch();
    let status = response.status();
    return (status, body_of(response));
}

#[test]
fn a_well_formed_body_reaches_the_route() {
    let client = client();
    let response = client.post("/stub/body").body(GOOD_VOUCHER).dispatch();

    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.into_string().as_deref(), Some("SAVE10"));
}

#[test]
fn a_field_of_the_wrong_type_is_422_naming_the_field() {
    let client = client();
    let cases = [
        (
            GOOD_VOUCHER.replace(r#""quota":5"#, r#""quota":"five""#),
            "quota",
        ),
        (
            GOOD_VOUCHER.replace(r#""quota":5"#, r#""quota":99999999999"#),
            "quota",
        ),
        (
            GOOD_VOUCHER.replace(r#""FIXED""#, r#""PERCENT""#),
            "discount_type",
        ),
        (GOOD_VOUCHER.replace(r#""10000""#, r#""ten""#), "value"),
        (
            GOOD_VOUCHER.replace("2099-01-01T00:00:00Z", "tomorrow"),
            "expires_at",
        ),
    ];

    for (body, field) in cases {
        let (status, answer) = body_answer(&client, &body);

        assert_eq!(status, Status::UnprocessableEntity, "{field}: {answer}");
        assert_eq!(answer["error"], "INVALID_BODY", "{field}: {answer}");
        let message = answer["message"].as_str().unwrap_or("");
        assert!(
            message.starts_with(&format!("{field}: ")),
            "{field}: {message}"
        );
    }
}

#[test]
fn a_missing_field_is_422_naming_the_field() {
    let client = client();
    let body = GOOD_VOUCHER.replace(r#""title":"Ten off","#, "");

    let (status, answer) = body_answer(&client, &body);

    assert_eq!(status, Status::UnprocessableEntity);
    assert_eq!(answer["error"], "INVALID_BODY");
    assert!(
        answer["message"].as_str().unwrap_or("").contains("`title`"),
        "{answer}"
    );
}

#[test]
fn a_body_that_is_not_json_is_400() {
    let client = client();

    for body in ["{", "not json", &format!("{GOOD_VOUCHER} trailing")] {
        let (status, answer) = body_answer(&client, body);
        assert_eq!(status, Status::BadRequest, "{body}");
        assert_eq!(answer["error"], "INVALID_JSON", "{body}");
    }
}

#[test]
fn a_body_over_the_json_limit_is_413_with_its_code() {
    let client = client();
    let response = client
        .post("/stub/body")
        .body(format!("[{}]", "0,".repeat(600_000)))
        .dispatch();

    assert_eq!(response.status(), Status::PayloadTooLarge);
    assert_eq!(body_of(response)["error"], "PAYLOAD_TOO_LARGE");
}
