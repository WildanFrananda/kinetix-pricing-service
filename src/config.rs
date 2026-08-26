use std::env;

pub struct AppConfig {
    pub database_url: String,
    pub port: u16,
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

        return AppConfig { database_url, port };
    }
}
