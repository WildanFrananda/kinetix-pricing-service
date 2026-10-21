use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use crate::models::{Discount, DiscountType, PriceItemRequest};
use crate::traits::discount_evaluator::DiscountEvaluator;

pub struct DefaultDiscountEvaluator;

impl DiscountEvaluator for DefaultDiscountEvaluator {
    fn matches_item(&self, discount: &Discount, item: &PriceItemRequest) -> bool {
        let matches_product = discount.target_product_id.as_ref().is_some_and(|p| {
            return p == &item.product_id;
        });

        let matches_category = discount.target_category_id.as_ref().is_some_and(|c| {
            return item.category_id.as_ref().is_some_and(|item_c| {
                return item_c == c;
            });
        });

        let is_global =
            discount.target_product_id.is_none() && discount.target_category_id.is_none();

        return matches_product || matches_category || is_global;
    }

    fn calculate_savings(&self, discount: &Discount, base_price: Decimal) -> Decimal {
        if base_price <= dec!(0) {
            return dec!(0.00);
        }

        let savings = match discount.get_discount_type() {
            Some(DiscountType::Percentage) => base_price * (discount.value / dec!(100.00)),
            Some(DiscountType::Fixed) => discount.value,
            None => dec!(0.00),
        };

        return savings.max(dec!(0.00)).min(base_price);
    }
}
