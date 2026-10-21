use rust_decimal::Decimal;

use crate::models::Voucher;

pub trait VoucherEvaluator: Send + Sync {
    fn is_eligible(&self, voucher: &Voucher, subtotal: Decimal) -> bool;
    fn calculate_discount(&self, voucher: &Voucher, subtotal: Decimal) -> Decimal;
}
