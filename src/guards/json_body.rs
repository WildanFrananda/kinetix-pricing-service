use rocket::data::{self, Data, FromData, Limits};
use rocket::http::Status;
use rocket::Request;
use serde::de::{DeserializeOwned, IgnoredAny};
use serde_json::error::Category;

use crate::error::AppError;
use crate::error_code;
use crate::guards::guard_refusal::GuardRefusal;

const ROOT_PATH: &str = ".";
const JSON_WHITESPACE: [char; 4] = [' ', '\t', '\n', '\r'];
const OBJECT_OPENING: char = '{';
const NOT_AN_OBJECT: &str = "the body must be a JSON object with this endpoint's fields by name.";

#[derive(Debug, Clone, PartialEq)]
pub struct JsonBody<T>(pub T);

impl<T> JsonBody<T> {
    pub fn into_inner(self) -> T {
        return self.0;
    }
}

#[rocket::async_trait]
impl<'r, T: DeserializeOwned + Send> FromData<'r> for JsonBody<T> {
    type Error = AppError;

    async fn from_data(req: &'r Request<'_>, data: Data<'r>) -> data::Outcome<'r, Self> {
        let limit = req.limits().get("json").unwrap_or(Limits::JSON);
        let text = match data.open(limit).into_string().await {
            Ok(text) if text.is_complete() => text.into_inner(),
            Ok(_) => {
                return refuse(
                    req,
                    Status::PayloadTooLarge,
                    error_code::PAYLOAD_TOO_LARGE,
                    &format!("the body is larger than the {limit} this endpoint reads"),
                );
            }
            Err(error) => {
                return refuse(
                    req,
                    Status::BadRequest,
                    error_code::INVALID_JSON,
                    &format!("the body could not be read: {error}"),
                );
            }
        };

        if !opens_an_object(&text) {
            match serde_json::from_str::<IgnoredAny>(&text) {
                Ok(_) => {
                    return refuse(
                        req,
                        Status::UnprocessableEntity,
                        error_code::INVALID_BODY,
                        NOT_AN_OBJECT,
                    );
                }
                Err(error) => {
                    return refuse(
                        req,
                        Status::BadRequest,
                        error_code::INVALID_JSON,
                        &format!("the body is not valid JSON: {error}"),
                    );
                }
            }
        }

        let mut deserializer = serde_json::Deserializer::from_str(&text);
        let parsed = serde_path_to_error::deserialize::<_, T>(&mut deserializer);

        match parsed.map(|value| return (value, deserializer.end())) {
            Ok((value, Ok(()))) => return data::Outcome::Success(JsonBody(value)),
            Ok((_, Err(trailing))) => {
                return refuse(
                    req,
                    Status::BadRequest,
                    error_code::INVALID_JSON,
                    &format!("the body is not valid JSON: {trailing}"),
                );
            }
            Err(error) => {
                let field = error.path().to_string();
                let cause = error.into_inner();

                if cause.classify() != Category::Data {
                    return refuse(
                        req,
                        Status::BadRequest,
                        error_code::INVALID_JSON,
                        &format!("the body is not valid JSON: {cause}"),
                    );
                }

                let message = match field.as_str() {
                    ROOT_PATH => format!("the body does not match this endpoint: {cause}"),
                    path => format!("{path}: {cause}"),
                };
                return refuse(
                    req,
                    Status::UnprocessableEntity,
                    error_code::INVALID_BODY,
                    &message,
                );
            }
        }
    }
}

fn opens_an_object(text: &str) -> bool {
    return text
        .trim_start_matches(JSON_WHITESPACE)
        .starts_with(OBJECT_OPENING);
}

fn refuse<'r, T>(
    req: &'r Request<'_>,
    status: Status,
    code: &'static str,
    message: &str,
) -> data::Outcome<'r, T, AppError> {
    GuardRefusal::record(req, code, message);
    return data::Outcome::Error((status, AppError::BadRequest(message.to_string())));
}
