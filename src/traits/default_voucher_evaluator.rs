use chrono::Utc;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use crate::models::{DiscountType, Voucher};
use crate::traits::voucher_evaluator::VoucherEvaluator;

pub struct DefaultVoucherEvaluator;

impl VoucherEvaluator for DefaultVoucherEvaluator {
    fn is_eligible(&self, voucher: &Voucher, subtotal: Decimal) -> bool {
        let now = Utc::now();
        let value_can_discount = match voucher.get_discount_type() {
            Some(DiscountType::Percentage) => voucher.value > dec!(0) && voucher.value <= dec!(100),
            Some(DiscountType::Fixed) => voucher.value > dec!(0),
            None => false,
        };
        let cap_can_discount = voucher.max_discount.is_none_or(|max| return max > dec!(0));

        let is_valid = voucher.active
            && value_can_discount
            && cap_can_discount
            && voucher.expires_at >= now
            && voucher.used_count < voucher.quota
            && subtotal > dec!(0)
            && subtotal >= voucher.min_spend;
        return is_valid;
    }

    fn calculate_discount(&self, voucher: &Voucher, subtotal: Decimal) -> Decimal {
        if !self.is_eligible(voucher, subtotal) {
            return dec!(0.00);
        }

        let raw_discount = match voucher.get_discount_type() {
            Some(DiscountType::Percentage) => subtotal * (voucher.value / dec!(100.00)),
            Some(DiscountType::Fixed) => voucher.value,
            None => return dec!(0.00),
        };

        let capped_discount = match voucher.max_discount {
            Some(max_discount) => raw_discount.min(max_discount),
            None => raw_discount,
        };

        return capped_discount.max(dec!(0.00)).min(subtotal);
    }
}
