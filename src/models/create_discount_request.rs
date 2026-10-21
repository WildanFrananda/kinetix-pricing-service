use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::models::discount_type::DiscountType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDiscountRequest {
    pub title: String,
    pub discount_type: DiscountType,
    pub value: Decimal,
    pub target_product_id: Option<String>,
    pub target_category_id: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
}
