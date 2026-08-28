use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use crate::models::{Discount, DiscountType, PriceItemRequest};

pub trait DiscountEvaluator: Send + Sync {
    fn matches_item(&self, discount: &Discount, item: &PriceItemRequest) -> bool;
    fn calculate_savings(&self, discount: &Discount, base_price: Decimal) -> Decimal;
}

pub struct DefaultDiscountEvaluator;

impl DiscountEvaluator for DefaultDiscountEvaluator {
    fn matches_item(&self, discount: &Discount, item: &PriceItemRequest) -> bool {
        let matches_product = discount
            .target_product_id
            .as_ref()
            .map_or(false, |p| {
                return p == &item.product_id;
            });

        let matches_category = discount
            .target_category_id
            .as_ref()
            .map_or(false, |c| {
                return item.category_id.as_ref().map_or(false, |item_c| {
                    return item_c == c;
                });
            });

        let is_global = discount.target_product_id.is_none() && discount.target_category_id.is_none();

        return matches_product || matches_category || is_global;
    }

    fn calculate_savings(&self, discount: &Discount, base_price: Decimal) -> Decimal {
        let savings = match discount.get_discount_type() {
            DiscountType::Percentage => base_price * (discount.value / dec!(100.00)),
            DiscountType::Fixed => discount.value.min(base_price),
        };
        return savings;
    }
}
