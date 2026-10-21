use rocket::http::Status;
use rocket::response::{self, Responder, Response};
use rocket::Request;

pub const REPLAYED_HEADER: &str = "Idempotent-Replayed";

#[derive(Debug)]
pub struct CreatedResponse<R> {
    pub body: R,
    pub replayed: bool,
}

impl<R> CreatedResponse<R> {
    pub fn new(body: R, replayed: bool) -> Self {
        return Self { body, replayed };
    }
}

impl<'r, 'o: 'r, R: Responder<'r, 'o>> Responder<'r, 'o> for CreatedResponse<R> {
    fn respond_to(self, request: &'r Request<'_>) -> response::Result<'o> {
        let mut response = Response::build_from(self.body.respond_to(request)?);
        response.status(Status::Created);

        if self.replayed {
            response.raw_header(REPLAYED_HEADER, "true");
        }

        return response.ok();
    }
}
