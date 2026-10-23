use rocket::Request;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdminActor(Option<String>);

impl AdminActor {
    pub fn record(request: &Request<'_>, principal_id: &str) {
        request.local_cache(|| return AdminActor(Some(principal_id.to_string())));
        return;
    }

    pub fn of<'r>(request: &'r Request<'_>) -> Option<&'r str> {
        return request.local_cache(|| return AdminActor(None)).0.as_deref();
    }
}
