use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::models::price_item_request::PriceItemRequest;
use crate::models::shipping_quote_request::ShippingQuoteRequest;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalculatePriceRequest {
    pub items: Vec<PriceItemRequest>,
    pub voucher_code: Option<String>,
    pub base_shipping_fee: Option<Decimal>,
    pub shipping: Option<ShippingQuoteRequest>,
    pub merchant_principal_id: Option<String>,
}
