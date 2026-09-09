use std::sync::Arc;

use tower_layer::Layer;

use super::grpc_metrics_service::GrpcMetricsService;
use super::metrics::Metrics;

#[derive(Clone)]
pub struct GrpcMetricsLayer {
    metrics: Arc<Metrics>,
}

impl GrpcMetricsLayer {
    pub fn new(metrics: Arc<Metrics>) -> Self {
        return GrpcMetricsLayer { metrics };
    }
}

impl<S> Layer<S> for GrpcMetricsLayer {
    type Service = GrpcMetricsService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        return GrpcMetricsService::new(inner, self.metrics.clone());
    }
}
