use diesel_async::RunQueryDsl;
use rocket::http::Status;
use rocket::serde::json::Value;
use rocket::{get, State};

use crate::DbPool;

#[get("/health")]
pub fn health_check() -> Value {
    return serde_json::json!({
        "status": "ok",
        "service": "kinetix-pricing-service"
    });
}

#[get("/health/ready")]
pub async fn health_ready(pool: &State<DbPool>) -> (Status, Value) {
    let mut connection = match pool.get().await {
        Ok(connection) => connection,
        Err(error) => {
            tracing::error!(error = %error, "readiness: the pool could not hand out a connection");
            return (Status::ServiceUnavailable, unavailable());
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
            tracing::error!(error = %error, "readiness: SELECT 1 failed");
            return (Status::ServiceUnavailable, unavailable());
        }
    }
}

fn unavailable() -> Value {
    return serde_json::json!({ "status": "unavailable", "database": "unreachable" });
}
