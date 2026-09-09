use std::sync::Arc;

use rocket::fairing::{Fairing, Info, Kind};
use rocket::{Data, Orbit, Request, Response, Rocket};

use super::metrics::Metrics;
use super::request_start::RequestStart;
use super::route_template::route_template;

pub const UNMATCHED_ROUTE: &str = "unmatched";

pub struct MetricsFairing {
    metrics: Arc<Metrics>,
}

impl MetricsFairing {
    pub fn new(metrics: Arc<Metrics>) -> Self {
        return MetricsFairing { metrics };
    }
}

#[rocket::async_trait]
impl Fairing for MetricsFairing {
    fn info(&self) -> Info {
        return Info {
            name: "metrics",
            kind: Kind::Liftoff | Kind::Request | Kind::Response,
        };
    }

    async fn on_liftoff(&self, rocket: &Rocket<Orbit>) {
        for route in rocket.routes() {
            self.metrics
                .seed_http_route(route.method.as_str(), &route_template(route.uri.path()));
        }
        return;
    }

    async fn on_request(&self, request: &mut Request<'_>, _data: &mut Data<'_>) {
        request.local_cache(RequestStart::now);
        return;
    }

    async fn on_response<'r>(&self, request: &'r Request<'_>, response: &mut Response<'r>) {
        let started = request.local_cache(RequestStart::now);

        let route = match request.route() {
            Some(route) => route_template(route.uri.path()),
            None => UNMATCHED_ROUTE.to_string(),
        };

        self.metrics.observe_http(
            request.method().as_str(),
            &route,
            response.status().code,
            started.0.elapsed().as_secs_f64(),
        );
        return;
    }
}
