use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::models::price_item_response::PriceItemResponse;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalculatePriceResponse {
    pub subtotal: Decimal,
    pub total_discount: Decimal,
    pub voucher_discount: Decimal,
    pub final_total: Decimal,
    pub applied_voucher: Option<String>,
    pub items: Vec<PriceItemResponse>,
    pub base_shipping_fee: Decimal,
    pub final_shipping_fee: Decimal,
}
