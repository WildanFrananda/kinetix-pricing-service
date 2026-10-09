use chrono::{DateTime, Utc};
use diesel::pg::Pg;
use diesel::prelude::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;

use crate::models::discount::DiscountType;
use crate::schema::vouchers;

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Selectable, Insertable)]
#[diesel(table_name = vouchers)]
#[diesel(check_for_backend(Pg))]
pub struct Voucher {
    pub id: Uuid,
    pub code: String,
    pub title: String,
    pub discount_type: String,
    pub value: Decimal,
    pub min_spend: Decimal,
    pub max_discount: Option<Decimal>,
    pub quota: i32,
    pub used_count: i32,
    pub active: bool,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub merchant_principal_id: Option<String>,
}

impl Voucher {
    pub fn get_discount_type(&self) -> Option<DiscountType> {
        return DiscountType::from_str(&self.discount_type).ok();
    }
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
    #[serde(default)]
    pub merchant_principal_id: Option<String>,
}

impl ApplyVoucherRequest {
    pub fn new(
        code: String,
        cart_subtotal: Decimal,
        merchant_principal_id: Option<String>,
    ) -> Self {
        return Self {
            code,
            cart_subtotal,
            merchant_principal_id,
        };
    }
}
