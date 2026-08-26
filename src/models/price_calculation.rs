use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceItemRequest {
    pub product_id: String,
    pub category_id: Option<String>,
    pub base_price: Decimal,
    pub quantity: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalculatePriceRequest {
    pub items: Vec<PriceItemRequest>,
    pub voucher_code: Option<String>,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalculatePriceResponse {
    pub subtotal: Decimal,
    pub total_discount: Decimal,
    pub voucher_discount: Decimal,
    pub final_total: Decimal,
    pub applied_voucher: Option<String>,
    pub items: Vec<PriceItemResponse>,
}
