use std::convert::Infallible;
use std::sync::{Arc, OnceLock};

use rocket::request::{FromRequest, Outcome, Request};

#[derive(Debug, Clone, Default)]
pub struct AdminActionTarget(Arc<OnceLock<String>>);

impl AdminActionTarget {
    pub fn of<'r>(request: &'r Request<'_>) -> &'r AdminActionTarget {
        return request.local_cache(AdminActionTarget::default);
    }

    pub fn name(&self, acted_on: &str) {
        let _ = self.0.set(acted_on.to_string());
        return;
    }

    pub fn named(&self) -> Option<&str> {
        return self.0.get().map(String::as_str);
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AdminActionTarget {
    type Error = Infallible;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        return Outcome::Success(AdminActionTarget::of(req).clone());
    }
}
