#[macro_use]
extern crate rocket;

use std::net::SocketAddr;
use diesel::{Connection, PgConnection};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use kinetix_pricing_service::config::AppConfig;
use kinetix_pricing_service::db::create_pool;
use kinetix_pricing_service::grpc::{PricingGrpcServer, PricingServiceServer};
use kinetix_pricing_service::routes::{
    discount_routes::{create_discount, list_discounts},
    flash_sale_routes::{create_flash_sale, get_flash_sale_for_product},
    health_routes::{health_check, health_ready},
    voucher_routes::{apply_voucher, create_voucher, get_voucher},
};
use tonic::transport::Server;
use tracing::info;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

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

    let db_pool = create_pool(&app_cfg.database_url);

    let grpc_pool = db_pool.clone();
    let grpc_addr: SocketAddr = format!("0.0.0.0:{}", app_cfg.grpc_port).parse()?;
    let grpc_service = PricingGrpcServer::new(grpc_pool);

    let reflection = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(tonic::include_file_descriptor_set!(
            "pricing_descriptor"
        ))
        .build()?;

    let grpc_server = async move {
        info!("gRPC Pricing Server listening on {}", grpc_addr);

        return Server::builder()
            .add_service(PricingServiceServer::new(grpc_service))
            .add_service(reflection)
            .serve(grpc_addr)
            .await
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                format!("gRPC server on {grpc_addr} stopped: {e}").into()
            });
    };

    let rest_server = async move {
        let _rocket = rocket::build()
            .manage(db_pool)
            .mount(
                "/",
                routes![
                    health_check,
                    health_ready,
                    create_discount,
                    list_discounts,
                    create_voucher,
                    apply_voucher,
                    get_voucher,
                    create_flash_sale,
                    get_flash_sale_for_product
                ],
            )
            .launch()
            .await
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                format!("REST server stopped: {e}").into()
            })?;

        return Ok(());
    };

    tokio::try_join!(grpc_server, rest_server)
        .map_err(|e| -> Box<dyn std::error::Error> { e.to_string().into() })?;

    return Ok(());
}
