use chrono::Utc;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use crate::models::{DiscountType, Voucher};

pub trait VoucherEvaluator: Send + Sync {
    fn is_eligible(&self, voucher: &Voucher, subtotal: Decimal) -> bool;
    fn calculate_discount(&self, voucher: &Voucher, subtotal: Decimal) -> Decimal;
}

pub struct DefaultVoucherEvaluator;

impl VoucherEvaluator for DefaultVoucherEvaluator {
    fn is_eligible(&self, voucher: &Voucher, subtotal: Decimal) -> bool {
        let now = Utc::now();
        let is_valid = voucher.active
            && voucher.expires_at >= now
            && voucher.used_count < voucher.quota
            && subtotal >= voucher.min_spend;
        return is_valid;
    }

    fn calculate_discount(&self, voucher: &Voucher, subtotal: Decimal) -> Decimal {
        if !self.is_eligible(voucher, subtotal) {
            return dec!(0.00);
        }

        let raw_discount = match voucher.discount_type {
            DiscountType::Percentage => subtotal * (voucher.value / dec!(100.00)),
            DiscountType::Fixed => voucher.value,
        };

        let bounded_discount = if let Some(max_d) = voucher.max_discount {
            raw_discount.min(max_d)
        } else {
            raw_discount
        };

        return bounded_discount.min(subtotal);
    }
}
