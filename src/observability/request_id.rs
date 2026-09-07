use tonic::Request;

pub const REQUEST_ID_KEY: &str = "x-request-id";

pub fn request_id<T>(request: &Request<T>) -> Option<String> {
    return request
        .metadata()
        .get(REQUEST_ID_KEY)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string());
}
