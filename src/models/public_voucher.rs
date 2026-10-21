use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::models::voucher::Voucher;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PublicVoucher {
    pub code: String,
    pub title: String,
    pub discount_type: String,
    pub value: Decimal,
    pub min_spend: Decimal,
    pub max_discount: Option<Decimal>,
    pub expires_at: DateTime<Utc>,
    pub merchant_principal_id: Option<String>,
}

impl From<Voucher> for PublicVoucher {
    fn from(voucher: Voucher) -> Self {
        return Self {
            code: voucher.code,
            title: voucher.title,
            discount_type: voucher.discount_type,
            value: voucher.value,
            min_spend: voucher.min_spend,
            max_discount: voucher.max_discount,
            expires_at: voucher.expires_at,
            merchant_principal_id: voucher.merchant_principal_id,
        };
    }
}
