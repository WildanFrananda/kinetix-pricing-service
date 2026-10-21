use chrono::{DateTime, Utc};
use diesel::pg::Pg;
use diesel::prelude::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;

use crate::models::discount_type::DiscountType;
use crate::schema::discounts;

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Selectable, Insertable)]
#[diesel(table_name = discounts)]
#[diesel(check_for_backend(Pg))]
pub struct Discount {
    pub id: Uuid,
    pub title: String,
    pub discount_type: String,
    pub value: Decimal,
    pub target_product_id: Option<String>,
    pub target_category_id: Option<String>,
    pub active: bool,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub merchant_principal_id: Option<String>,
}

impl Discount {
    pub fn get_discount_type(&self) -> Option<DiscountType> {
        return DiscountType::from_str(&self.discount_type).ok();
    }
}
