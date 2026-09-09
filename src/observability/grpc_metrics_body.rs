use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use http::HeaderMap;
use http_body::{Body, SizeHint};
use tonic::Status;

use super::grpc_labels::{grpc_code_label, NO_GRPC_STATUS};
use super::metrics::Metrics;

pub struct GrpcMetricsBody<B> {
    inner: B,
    pending: Option<(Arc<Metrics>, &'static str)>,
}

impl<B> GrpcMetricsBody<B> {
    pub fn recording(inner: B, metrics: Arc<Metrics>, grpc_method: &'static str) -> Self {
        return GrpcMetricsBody {
            inner,
            pending: Some((metrics, grpc_method)),
        };
    }

    pub fn already_recorded(inner: B) -> Self {
        return GrpcMetricsBody {
            inner,
            pending: None,
        };
    }

    fn record(&mut self, grpc_code: &str) {
        if let Some((metrics, grpc_method)) = self.pending.take() {
            metrics.observe_grpc_server_call(grpc_method, grpc_code);
        }
        return;
    }
}

impl<B> Body for GrpcMetricsBody<B>
where
    B: Body + Unpin,
{
    type Data = B::Data;
    type Error = B::Error;

    fn poll_data(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Self::Data, Self::Error>>> {
        return Pin::new(&mut self.inner).poll_data(cx);
    }

    fn poll_trailers(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<Option<HeaderMap>, Self::Error>> {
        let polled = Pin::new(&mut self.inner).poll_trailers(cx);

        if let Poll::Ready(Ok(ref trailers)) = polled {
            let code = trailers
                .as_ref()
                .and_then(Status::from_header_map)
                .map(|status| grpc_code_label(status.code()))
                .unwrap_or(NO_GRPC_STATUS);
            self.record(code);
        }

        return polled;
    }

    fn is_end_stream(&self) -> bool {
        return self.inner.is_end_stream();
    }

    fn size_hint(&self) -> SizeHint {
        return self.inner.size_hint();
    }
}

impl<B> Drop for GrpcMetricsBody<B> {
    fn drop(&mut self) {
        self.record(NO_GRPC_STATUS);
        return;
    }
}
