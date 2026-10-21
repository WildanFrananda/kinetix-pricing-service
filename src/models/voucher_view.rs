use serde::Serialize;

use crate::models::public_voucher::PublicVoucher;
use crate::models::voucher::Voucher;

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum VoucherView {
    Full(Voucher),
    Public(PublicVoucher),
}
