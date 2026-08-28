use std::fmt;
use std::str::FromStr;
use chrono::{DateTime, Utc};
use diesel::pg::Pg;
use diesel::prelude::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;
use crate::schema::discounts;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiscountType {
    Percentage,
    Fixed,
}

impl fmt::Display for DiscountType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl DiscountType {
    pub fn as_str(&self) -> &'static str {
        match self {
            DiscountType::Percentage => return "PERCENTAGE",
            DiscountType::Fixed => return "FIXED",
        }
    }
}

impl FromStr for DiscountType {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_uppercase().as_str() {
            "PERCENTAGE" => return Ok(DiscountType::Percentage),
            "FIXED" => return Ok(DiscountType::Fixed),
            other => return Err(AppError::BadRequest(format!("Invalid discount_type: '{}'. Expected 'PERCENTAGE' or 'FIXED'", other))),
        }
    }
}

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
}

impl Discount {
    pub fn get_discount_type(&self) -> DiscountType {
        return DiscountType::from_str(&self.discount_type).unwrap_or(DiscountType::Percentage);
    }
}

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
