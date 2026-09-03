use diesel_async::RunQueryDsl;
use rocket::http::Status;
use rocket::serde::json::Value;
use rocket::{get, State};

use crate::DbPool;

#[get("/health")]
pub fn health_check() -> Value {
    return serde_json::json!({
        "status": "ok",
        "service": "kinetix-pricing-service",
        "framework": "Rocket 0.5 (Rust)",
        "mode": "production",
        "port": 6000
    });
}

#[get("/health/ready")]
pub async fn health_ready(pool: &State<DbPool>) -> (Status, Value) {
    let mut connection = match pool.get().await {
        Ok(connection) => connection,
        Err(error) => {
            return (
                Status::ServiceUnavailable,
                serde_json::json!({
                    "status": "unavailable",
                    "database": "unreachable",
                    "detail": error.to_string()
                }),
            );
        }
    };

    match diesel::sql_query("SELECT 1").execute(&mut connection).await {
        Ok(_) => {
            return (
                Status::Ok,
                serde_json::json!({ "status": "ok", "database": "reachable" }),
            );
        }
        Err(error) => {
            return (
                Status::ServiceUnavailable,
                serde_json::json!({
                    "status": "unavailable",
                    "database": "unreachable",
                    "detail": error.to_string()
                }),
            );
        }
    }
}
