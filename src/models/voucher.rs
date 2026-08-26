use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::discount::DiscountType;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Voucher {
    pub id: Uuid,
    pub code: String,
    pub title: String,
    pub discount_type: DiscountType,
    pub value: Decimal,
    pub min_spend: Decimal,
    pub max_discount: Option<Decimal>,
    pub quota: i32,
    pub used_count: i32,
    pub active: bool,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyVoucherRequest {
    pub code: String,
    pub cart_subtotal: Decimal,
}

impl ApplyVoucherRequest {
    pub fn new(code: String, cart_subtotal: Decimal) -> Self {
        return Self { code, cart_subtotal };
    }
}
