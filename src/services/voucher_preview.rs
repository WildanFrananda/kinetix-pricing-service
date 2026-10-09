use rust_decimal::Decimal;

use crate::models::Voucher;
use crate::traits::{applies_to_cart, VoucherEvaluator};

pub fn previewed(
    voucher: Voucher,
    evaluator: &dyn VoucherEvaluator,
    cart_subtotal: Decimal,
    cart_merchant: Option<&str>,
) -> Option<Voucher> {
    if !evaluator.is_eligible(&voucher, cart_subtotal) {
        return None;
    }
    if !applies_to_cart(voucher.merchant_principal_id.as_deref(), cart_merchant) {
        return None;
    }
    return Some(voucher);
}
