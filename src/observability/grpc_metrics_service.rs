use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use http::{Request, Response};
use tonic::Status;
use tower_service::Service;

use super::grpc_labels::{grpc_code_label, grpc_method_label};
use super::grpc_metrics_body::GrpcMetricsBody;
use super::metrics::Metrics;

#[derive(Clone)]
pub struct GrpcMetricsService<S> {
    inner: S,
    metrics: Arc<Metrics>,
}

impl<S> GrpcMetricsService<S> {
    pub fn new(inner: S, metrics: Arc<Metrics>) -> Self {
        return GrpcMetricsService { inner, metrics };
    }
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for GrpcMetricsService<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>>,
    S::Future: Send + 'static,
    ResBody: 'static, {
    type Response = Response<GrpcMetricsBody<ResBody>>;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        return self.inner.poll_ready(cx);
    }

    fn call(&mut self, request: Request<ReqBody>) -> Self::Future {
        let grpc_method = grpc_method_label(request.uri().path());
        let metrics = self.metrics.clone();
        let inner = self.inner.call(request);

        return Box::pin(async move {
            let response = inner.await?;

            if let Some(status) = Status::from_header_map(response.headers()) {
                metrics.observe_grpc_server_call(grpc_method, grpc_code_label(status.code()));
                return Ok(response.map(GrpcMetricsBody::already_recorded));
            }

            return Ok(response.map(move |body| {
                return GrpcMetricsBody::recording(body, metrics, grpc_method);
            }));
        });
    }
}
