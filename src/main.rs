#[macro_use]
extern crate rocket;

use std::net::SocketAddr;
use kinetix_pricing_service::config::AppConfig;
use kinetix_pricing_service::db::create_pool;
use kinetix_pricing_service::grpc::{PricingGrpcServer, PricingServiceServer};
use kinetix_pricing_service::routes::{
    discount_routes::{create_discount, list_discounts},
    flash_sale_routes::{create_flash_sale, get_flash_sale_for_product},
    health_routes::health_check,
    voucher_routes::{apply_voucher, create_voucher, get_voucher},
};
use tonic::transport::Server;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let app_cfg = AppConfig::load();
    info!("Initializing Kinetix Pricing Service on port {} (REST Admin) and :50054 (gRPC with Diesel Async)", app_cfg.port);

    let db_pool = create_pool(&app_cfg.database_url);

    // Spawn gRPC Server on port 50054
    let grpc_pool = db_pool.clone();
    let grpc_addr: SocketAddr = "0.0.0.0:50054".parse()?;
    let grpc_service = PricingGrpcServer::new(grpc_pool);

    tokio::spawn(async move {
        info!("gRPC Pricing Server listening on {}", grpc_addr);
        if let Err(e) = Server::builder()
            .add_service(PricingServiceServer::new(grpc_service))
            .serve(grpc_addr)
            .await
        {
            eprintln!("gRPC Server error: {}", e);
        }
    });

    // Run Rocket REST Admin Server on port 6000
    let _rocket_server = rocket::build()
        .manage(db_pool)
        .mount(
            "/",
            routes![
                health_check,
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
        .await?;

    return Ok(());
}
