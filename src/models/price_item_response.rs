use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceItemResponse {
    pub product_id: String,
    pub base_price: Decimal,
    pub final_unit_price: Decimal,
    pub quantity: i32,
    pub line_total: Decimal,
    pub applied_flash_sale: Option<String>,
    pub applied_discount: Option<String>,
}
