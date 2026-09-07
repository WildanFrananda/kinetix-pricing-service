use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::Header;
use rocket::{Data, Request, Response};

use super::request_id::REQUEST_ID_KEY;

pub struct RequestIdFairing;

#[rocket::async_trait]
impl Fairing for RequestIdFairing {
    fn info(&self) -> Info {
        return Info {
            name: "correlation id",
            kind: Kind::Request | Kind::Response,
        };
    }

    async fn on_request(&self, request: &mut Request<'_>, _data: &mut Data<'_>) {
        tracing::info!(
            method = %request.method(),
            path = %request.uri().path(),
            request_id = request.headers().get_one(REQUEST_ID_KEY).unwrap_or("-"),
            "HTTP request"
        );
        return;
    }

    async fn on_response<'r>(&self, request: &'r Request<'_>, response: &mut Response<'r>) {
        if let Some(id) = request.headers().get_one(REQUEST_ID_KEY) {
            response.set_header(Header::new("X-Request-Id", id.to_string()));
        }
        return;
    }
}
