use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::Server;

use kinetix_pricing_service::db::create_pool;
use kinetix_pricing_service::grpc::{PricingGrpcServer, PricingServiceServer};
use kinetix_pricing_service::observability::{GrpcMetricsLayer, Metrics, ShutdownSignal};
use kinetix_pricing_service::proto::pricing::v1::pricing_service_client::PricingServiceClient;
use kinetix_pricing_service::proto::pricing::v1::RedeemVoucherRequest;
use kinetix_pricing_service::security::PeerGuard;
use kinetix_pricing_service::DbPool;

const CALCULATE: &str = "pricing.v1.PricingService/CalculatePrice";
const REDEEM: &str = "pricing.v1.PricingService/RedeemVoucher";

fn unconnected_pool() -> DbPool {
    return create_pool("postgres://unused:unused@127.0.0.1:1/unused");
}

fn metrics() -> Arc<Metrics> {
    return Arc::new(
        Metrics::new("kinetix-pricing-service", "0.1.0").expect("the metric registry was refused"),
    );
}

fn blank_redemption() -> RedeemVoucherRequest {
    return RedeemVoucherRequest {
        voucher_code: String::new(),
        order_number: String::new(),
        customer_principal_id: String::new(),
        idempotency_key: None,
    };
}

fn series(body: &str, grpc_method: &str, grpc_code: &str) -> u64 {
    let prefix = format!(
        r#"kinetix_grpc_server_calls_total{{grpc_code="{grpc_code}",grpc_method="{grpc_method}"}} "#
    );
    return body
        .lines()
        .find(|line| line.starts_with(&prefix))
        .map(|line| {
            return line[prefix.len()..]
                .trim()
                .parse::<u64>()
                .expect("the counter is not a whole number");
        })
        .unwrap_or_else(|| panic!("no series for {grpc_method} / {grpc_code}:\n{body}"));
}

async fn listener() -> (TcpListener, SocketAddr) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("could not bind a test port");
    let address = listener.local_addr().expect("the test port has no address");
    return (listener, address);
}

#[tokio::test]
async fn a_call_that_reaches_a_handler_is_counted_from_its_trailers() {
    let metrics = metrics();
    let shutdown = ShutdownSignal::new();
    let (listener, address) = listener().await;

    let server = {
        let metrics = metrics.clone();
        let shutdown = shutdown.clone();
        tokio::spawn(async move {
            Server::builder()
                .layer(GrpcMetricsLayer::new(metrics))
                .add_service(PricingServiceServer::new(PricingGrpcServer::new(
                    unconnected_pool(),
                )))
                .serve_with_incoming_shutdown(TcpListenerStream::new(listener), shutdown.wait())
                .await
                .expect("the test gRPC server stopped badly");
        })
    };

    let mut client = PricingServiceClient::connect(format!("http://{address}"))
        .await
        .expect("the test client could not connect");

    let response = client
        .redeem_voucher(blank_redemption())
        .await
        .expect("the call did not complete");
    assert!(!response.into_inner().success);

    let body = metrics.encode().expect("the registry would not encode");
    assert_eq!(series(&body, REDEEM, "OK"), 1);
    assert_eq!(series(&body, CALCULATE, "OK"), 0);

    shutdown.trigger("the test is over");
    server.await.expect("the test server task panicked");
}

#[tokio::test]
async fn a_call_the_peer_guard_turns_away_is_counted_too() {
    std::env::set_var("KINETIX_GRPC_ALLOWED_PEERS", "kinetix-order-service");
    let guard = PeerGuard::from_env().expect("the guard would not read its allow list");

    let metrics = metrics();
    let shutdown = ShutdownSignal::new();
    let (listener, address) = listener().await;

    let server = {
        let metrics = metrics.clone();
        let shutdown = shutdown.clone();
        tokio::spawn(async move {
            Server::builder()
                .layer(GrpcMetricsLayer::new(metrics))
                .layer(tonic::service::interceptor(move |req| guard.check(req)))
                .add_service(PricingServiceServer::new(PricingGrpcServer::new(
                    unconnected_pool(),
                )))
                .serve_with_incoming_shutdown(TcpListenerStream::new(listener), shutdown.wait())
                .await
                .expect("the test gRPC server stopped badly");
        })
    };

    let mut client = PricingServiceClient::connect(format!("http://{address}"))
        .await
        .expect("the test client could not connect");

    let status = client
        .redeem_voucher(blank_redemption())
        .await
        .expect_err("an uncertified caller was let through");
    assert_eq!(status.code(), tonic::Code::Unauthenticated);

    let body = metrics.encode().expect("the registry would not encode");
    assert_eq!(series(&body, REDEEM, "UNAUTHENTICATED"), 1);
    assert_eq!(series(&body, REDEEM, "OK"), 0);

    shutdown.trigger("the test is over");
    server.await.expect("the test server task panicked");
}

#[tokio::test]
async fn the_shutdown_signal_drains_the_grpc_server_and_lets_it_return() {
    let metrics = metrics();
    let shutdown = ShutdownSignal::new();
    let (listener, address) = listener().await;

    let server = {
        let metrics = metrics.clone();
        let shutdown = shutdown.clone();
        tokio::spawn(async move {
            Server::builder()
                .layer(GrpcMetricsLayer::new(metrics))
                .add_service(PricingServiceServer::new(PricingGrpcServer::new(
                    unconnected_pool(),
                )))
                .serve_with_incoming_shutdown(TcpListenerStream::new(listener), shutdown.wait())
                .await
                .expect("the test gRPC server stopped badly");
        })
    };

    let mut client = PricingServiceClient::connect(format!("http://{address}"))
        .await
        .expect("the test client could not connect");
    client
        .redeem_voucher(blank_redemption())
        .await
        .expect("the call did not complete");

    drop(client);
    shutdown.trigger("SIGTERM");

    tokio::time::timeout(std::time::Duration::from_secs(10), server)
        .await
        .expect("the gRPC server did not return after the signal fired")
        .expect("the test server task panicked");
}
