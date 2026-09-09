#[macro_use]
extern crate rocket;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use diesel::{Connection, PgConnection};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use kinetix_pricing_service::config::AppConfig;
use kinetix_pricing_service::db::create_pool;
use kinetix_pricing_service::grpc::{PricingGrpcServer, PricingServiceServer};
use kinetix_pricing_service::observability::{
    json_logging, GrpcMetricsLayer, Metrics, MetricsFairing, RequestIdFairing, ShutdownSignal,
};
use kinetix_pricing_service::routes::{
    catchers::{internal_error, not_found, other, unprocessable},
    discount_routes::{create_discount, list_discounts},
    flash_sale_routes::{create_flash_sale, get_flash_sale_for_product},
    health_routes::{health_check, health_ready},
    metrics_routes::metrics as metrics_endpoint,
    voucher_routes::{apply_voucher, create_voucher, get_voucher},
};
use kinetix_pricing_service::security::jwt::JwtVerifier;
use kinetix_pricing_service::security::{PeerGuard, ServiceIdentity};
use tonic::transport::Server;
use tracing::{error, info, warn};

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

const DRAIN_BUDGET: Duration = Duration::from_secs(25);

type ServerError = Box<dyn std::error::Error + Send + Sync>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    json_logging::init();

    if std::env::args().any(|arg| arg == "--migrate") {
        let database_url = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL environment variable MUST be configured. Fail-fast shutdown.");
        let mut connection = PgConnection::establish(&database_url)?;
        let applied = connection
            .run_pending_migrations(MIGRATIONS)
            .map_err(|e| -> Box<dyn std::error::Error> { format!("migration failed: {e}").into() })?;

        for migration in &applied {
            info!("applied migration {}", migration);
        }
        info!("migrations complete: {} applied", applied.len());

        return Ok(());
    }

    let app_cfg = AppConfig::load();
    info!(
        "Initializing Kinetix Pricing Service on port {} (REST Admin) and :{} (gRPC with Diesel Async)",
        app_cfg.port, app_cfg.grpc_port
    );

    let metrics = Metrics::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
        .map(Arc::new)
        .map_err(|e| -> Box<dyn std::error::Error> {
            format!("the metric registry was refused: {e}").into()
        })?;

    let db_pool = create_pool(&app_cfg.database_url);

    let grpc_pool = db_pool.clone();
    let grpc_addr: SocketAddr = format!("0.0.0.0:{}", app_cfg.grpc_port).parse()?;
    let grpc_service = PricingGrpcServer::new(grpc_pool);

    let reflection = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(tonic::include_file_descriptor_set!(
            "pricing_descriptor"
        ))
        .build()?;

    let service_identity = ServiceIdentity::load()
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    let server_tls = service_identity.server_tls();

    let peer_guard = PeerGuard::from_env().map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    let shutdown = ShutdownSignal::new();
    shutdown
        .listen_for_signals()
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    let grpc_server = {
        let shutdown = shutdown.clone();
        let metrics = metrics.clone();
        async move {
            let result: Result<(), ServerError> = async {
                info!("gRPC Pricing Server listening on {} (mTLS)", grpc_addr);

                return Server::builder()
                    .tls_config(server_tls)
                    .map_err(|e| -> ServerError {
                        format!("the service certificate was rejected: {e}").into()
                    })?
                    .layer(GrpcMetricsLayer::new(metrics))
                    .layer(tonic::service::interceptor(move |req| {
                        return peer_guard.check(req);
                    }))
                    .add_service(PricingServiceServer::new(grpc_service))
                    .add_service(reflection)
                    .serve_with_shutdown(grpc_addr, shutdown.wait())
                    .await
                    .map_err(|e| -> ServerError {
                        format!("gRPC server on {grpc_addr} stopped: {e}").into()
                    });
            }
            .await;

            shutdown.trigger("the gRPC server stopped");
            return result;
        }
    };

    let verifier = JwtVerifier::from_env().map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    let key_count = verifier
        .refresh()
        .await
        .map_err(|e| -> Box<dyn std::error::Error> { format!("cannot load identity's JWKS: {e}").into() })?;
    info!("loaded {} signing key(s) from identity's JWKS", key_count);

    let rest_server = {
        let shutdown = shutdown.clone();
        let metrics = metrics.clone();
        async move {
            let result: Result<(), ServerError> = async {
                let rocket = rocket::build()
                    .attach(RequestIdFairing)
                    .attach(MetricsFairing::new(metrics.clone()))
                    .register(
                        "/",
                        catchers![not_found, unprocessable, internal_error, other],
                    )
                    .manage(db_pool)
                    .manage(verifier)
                    .manage(metrics)
                    .mount(
                        "/",
                        routes![
                            health_check,
                            health_ready,
                            metrics_endpoint,
                            create_discount,
                            list_discounts,
                            create_voucher,
                            apply_voucher,
                            get_voucher,
                            create_flash_sale,
                            get_flash_sale_for_product
                        ],
                    )
                    .ignite()
                    .await
                    .map_err(|e| -> ServerError {
                        format!("REST server did not ignite: {e}").into()
                    })?;

                let handle = rocket.shutdown();
                let notified = shutdown.clone();
                tokio::spawn(async move {
                    notified.wait().await;
                    handle.notify();
                });

                rocket
                    .launch()
                    .await
                    .map_err(|e| -> ServerError { format!("REST server stopped: {e}").into() })?;

                return Ok(());
            }
            .await;

            shutdown.trigger("the REST server stopped");
            return result;
        }
    };

    let servers = async {
        return tokio::join!(grpc_server, rest_server);
    };

    let budget = {
        let shutdown = shutdown.clone();
        async move {
            shutdown.wait().await;
            tokio::time::sleep(DRAIN_BUDGET).await;
        }
    };

    tokio::select! {
        (grpc_result, rest_result) = servers => {
            if let Err(ref error) = grpc_result {
                error!(error = %error, "the gRPC server stopped with an error");
            }
            if let Err(ref error) = rest_result {
                error!(error = %error, "the REST server stopped with an error");
            }
            grpc_result.map_err(|e| -> Box<dyn std::error::Error> { e.to_string().into() })?;
            rest_result.map_err(|e| -> Box<dyn std::error::Error> { e.to_string().into() })?;
            info!("both servers drained; exiting");
            return Ok(());
        }
        _ = budget => {
            warn!(
                budget_seconds = DRAIN_BUDGET.as_secs(),
                "the drain budget ran out with work still in flight; exiting without finishing it"
            );
            return Err("the drain budget expired before both servers had finished".into());
        }
    }
}
