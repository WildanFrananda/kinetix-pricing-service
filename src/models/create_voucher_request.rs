use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::models::discount_type::DiscountType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVoucherRequest {
    pub code: String,
    pub title: String,
    pub discount_type: DiscountType,
    pub value: Decimal,
    pub min_spend: Decimal,
    pub max_discount: Option<Decimal>,
    pub quota: i32,
    pub expires_at: DateTime<Utc>,
}
