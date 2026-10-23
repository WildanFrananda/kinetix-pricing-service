use std::sync::Arc;

use rocket::fairing::{Fairing, Info, Kind};
use rocket::{Request, Response};

use super::admin_action_outcome::AdminActionOutcome;
use super::admin_actions::{admin_action_of_route, NO_TARGET};
use super::admin_actor::AdminActor;
use super::admin_audit::record_admin_action;
use super::metrics::Metrics;
use super::request_id::REQUEST_ID_KEY;
use crate::guards::AdminActionTarget;

pub struct AdminAuditFairing {
    metrics: Arc<Metrics>,
}

impl AdminAuditFairing {
    pub fn new(metrics: Arc<Metrics>) -> Self {
        return AdminAuditFairing { metrics };
    }
}

#[rocket::async_trait]
impl Fairing for AdminAuditFairing {
    fn info(&self) -> Info {
        return Info {
            name: "admin audit",
            kind: Kind::Response,
        };
    }

    async fn on_response<'r>(&self, request: &'r Request<'_>, response: &mut Response<'r>) {
        let action = request
            .route()
            .and_then(|route| return route.name.as_deref())
            .and_then(admin_action_of_route);
        let (Some(action), Some(actor)) = (action, AdminActor::of(request)) else {
            return;
        };

        record_admin_action(
            &self.metrics,
            actor,
            action,
            AdminActionTarget::of(request).named().unwrap_or(NO_TARGET),
            AdminActionOutcome::for_status(response.status()),
            request.headers().get_one(REQUEST_ID_KEY).unwrap_or("-"),
        );
        return;
    }
}
