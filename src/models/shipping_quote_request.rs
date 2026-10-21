use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShippingQuoteRequest {
    pub service_tier: String,
    pub distance_km: Decimal,
    pub total_weight_grams: i64,
}
