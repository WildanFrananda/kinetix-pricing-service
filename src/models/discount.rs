use std::fmt;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiscountType {
    Percentage,
    Fixed,
}

impl fmt::Display for DiscountType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiscountType::Percentage => write!(f, "PERCENTAGE"),
            DiscountType::Fixed => write!(f, "FIXED"),
        }
    }
}

impl DiscountType {
    pub fn as_str(&self) -> &'static str {
        match self {
            DiscountType::Percentage => return "PERCENTAGE",
            DiscountType::Fixed => return "FIXED",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "FIXED" => return DiscountType::Fixed,
            _ => return DiscountType::Percentage,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Discount {
    pub id: Uuid,
    pub title: String,
    pub discount_type: DiscountType,
    pub value: Decimal,
    pub target_product_id: Option<String>,
    pub target_category_id: Option<String>,
    pub active: bool,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discount_type_conversions() {
        assert_eq!(DiscountType::Percentage.as_str(), "PERCENTAGE");
        assert_eq!(DiscountType::Fixed.as_str(), "FIXED");
        assert_eq!(DiscountType::from_str("FIXED"), DiscountType::Fixed);
        assert_eq!(DiscountType::from_str("PERCENTAGE"), DiscountType::Percentage);
        assert_eq!(DiscountType::from_str("UNKNOWN"), DiscountType::Percentage);
    }
}
