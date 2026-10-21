use rocket::Request;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardRefusal {
    pub error: &'static str,
    pub message: String,
}

impl GuardRefusal {
    pub fn record(request: &Request<'_>, error: &'static str, message: &str) {
        request.local_cache(|| {
            return Some(GuardRefusal {
                error,
                message: message.to_string(),
            });
        });
    }

    pub fn of<'r>(request: &'r Request<'_>) -> Option<&'r GuardRefusal> {
        return request.local_cache(|| return None::<GuardRefusal>).as_ref();
    }
}
