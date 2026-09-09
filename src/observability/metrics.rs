use prometheus::{
    Encoder, HistogramOpts, HistogramVec, IntCounterVec, IntGaugeVec, Opts, Registry, TextEncoder,
};

use super::grpc_labels::KNOWN_GRPC_METHODS;

pub struct Metrics {
    registry: Registry,
    http_requests: IntCounterVec,
    http_duration: HistogramVec,
    grpc_server_calls: IntCounterVec,
}

impl Metrics {
    pub fn new(service: &str, version: &str) -> Result<Self, prometheus::Error> {
        let registry = Registry::new();

        let http_requests = IntCounterVec::new(
            Opts::new(
                "kinetix_http_requests_total",
                "HTTP requests this service has answered.",
            ),
            &["method", "route", "status"],
        )?;

        let http_duration = HistogramVec::new(
            HistogramOpts::new(
                "kinetix_http_request_duration_seconds",
                "How long this service took to answer an HTTP request, in seconds.",
            ),
            &["method", "route"],
        )?;

        let grpc_server_calls = IntCounterVec::new(
            Opts::new(
                "kinetix_grpc_server_calls_total",
                "gRPC calls this service has served.",
            ),
            &["grpc_method", "grpc_code"],
        )?;

        let build_info = IntGaugeVec::new(
            Opts::new(
                "kinetix_build_info",
                "Always 1. The labels are the payload.",
            ),
            &["service", "version"],
        )?;
        build_info.with_label_values(&[service, version]).set(1);

        registry.register(Box::new(http_requests.clone()))?;
        registry.register(Box::new(http_duration.clone()))?;
        registry.register(Box::new(grpc_server_calls.clone()))?;
        registry.register(Box::new(build_info))?;

        let metrics = Metrics {
            registry,
            http_requests,
            http_duration,
            grpc_server_calls,
        };

        for method in KNOWN_GRPC_METHODS {
            metrics.seed_grpc_method(method);
        }

        return Ok(metrics);
    }

    fn seed_grpc_method(&self, grpc_method: &str) {
        self.grpc_server_calls
            .with_label_values(&[grpc_method, "OK"]);
        return;
    }

    pub fn seed_http_route(&self, method: &str, route: &str) {
        self.http_requests
            .with_label_values(&[method, route, "200"]);
        self.http_duration.with_label_values(&[method, route]);
        return;
    }

    pub fn observe_http(&self, method: &str, route: &str, status: u16, seconds: f64) {
        let status = status.to_string();
        self.http_requests
            .with_label_values(&[method, route, &status])
            .inc();
        self.http_duration
            .with_label_values(&[method, route])
            .observe(seconds);
        return;
    }

    pub fn observe_grpc_server_call(&self, grpc_method: &str, grpc_code: &str) {
        self.grpc_server_calls
            .with_label_values(&[grpc_method, grpc_code])
            .inc();
        return;
    }

    pub fn encode(&self) -> Result<String, prometheus::Error> {
        let families = self.registry.gather();
        let mut body = Vec::new();
        TextEncoder::new().encode(&families, &mut body)?;
        return String::from_utf8(body).map_err(|error| {
            prometheus::Error::Msg(format!("the exposition is not UTF-8: {error}"))
        });
    }
}
