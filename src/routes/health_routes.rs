use rocket::get;
use rocket::serde::json::Value;

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
