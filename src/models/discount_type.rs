use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Result as FResult};
use std::str::FromStr;

use crate::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiscountType {
    Percentage,
    Fixed,
}

impl Display for DiscountType {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        return write!(f, "{}", self.as_str());
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
            other => {
                return Err(AppError::BadRequest(format!(
                    "Invalid discount_type: '{other}'. Expected 'PERCENTAGE' or 'FIXED'"
                )));
            }
        }
    }
}
