use std::env;

pub struct AppConfig {
    pub database_url: String,
    pub port: u16,
    pub grpc_port: u16,
    pub identity_grpc_url: String,
    pub catalog_grpc_url: String,
}

impl AppConfig {
    pub fn load() -> Self {
        dotenvy::dotenv().ok();

        let database_url = env::var("DATABASE_URL")
            .expect("DATABASE_URL environment variable MUST be configured. Fail-fast shutdown.");

        let port = env::var("PORT")
            .ok()
            .and_then(|p| {
                return p.parse::<u16>().ok();
            })
            .unwrap_or(6000);

        let grpc_port = env::var("GRPC_PORT")
            .expect("GRPC_PORT environment variable MUST be configured. Fail-fast shutdown.")
            .parse::<u16>()
            .expect("GRPC_PORT must be a port number. Fail-fast shutdown.");

        let identity_grpc_url = env::var("IDENTITY_GRPC_URL").expect(
            "IDENTITY_GRPC_URL environment variable MUST be configured. Fail-fast shutdown.",
        );

        let catalog_grpc_url = env::var("CATALOG_GRPC_URL").expect(
            "CATALOG_GRPC_URL environment variable MUST be configured. Fail-fast shutdown.",
        );

        return AppConfig {
            database_url,
            port,
            grpc_port,
            identity_grpc_url,
            catalog_grpc_url,
        };
    }
}
