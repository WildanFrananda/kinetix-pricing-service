use std::sync::Arc;

use rocket::http::{ContentType, Status};
use rocket::{get, State};

use crate::observability::Metrics;

#[get("/metrics")]
pub fn metrics(metrics: &State<Arc<Metrics>>) -> (Status, (ContentType, String)) {
    return match metrics.encode() {
        Ok(exposition) => (Status::Ok, (prometheus_text(), exposition)),
        Err(error) => {
            tracing::error!(error = %error, "the metric registry would not encode");
            (
                Status::InternalServerError,
                (
                    ContentType::Plain,
                    "the metric registry could not be encoded; this scrape says nothing about \
                     what the service has served\n"
                        .to_string(),
                ),
            )
        }
    };
}

fn prometheus_text() -> ContentType {
    return ContentType::new("text", "plain")
        .with_params([("version", "0.0.4"), ("charset", "utf-8")]);
}
