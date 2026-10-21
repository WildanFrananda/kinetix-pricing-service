use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceItemRequest {
    pub product_id: String,
    pub category_id: Option<String>,
    pub base_price: Decimal,
    pub quantity: i32,
}
