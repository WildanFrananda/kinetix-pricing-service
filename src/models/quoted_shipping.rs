use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotedShipping {
    pub service_tier: String,
    pub base_shipping_fee: Decimal,
    pub priced: bool,
}
