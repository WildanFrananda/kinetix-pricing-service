use rust_decimal::Decimal;

use crate::models::{Discount, PriceItemRequest};

pub trait DiscountEvaluator: Send + Sync {
    fn matches_item(&self, discount: &Discount, item: &PriceItemRequest) -> bool;
    fn calculate_savings(&self, discount: &Discount, base_price: Decimal) -> Decimal;
}
