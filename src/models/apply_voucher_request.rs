use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyVoucherRequest {
    pub code: String,
    pub cart_subtotal: Decimal,
    #[serde(default)]
    pub merchant_principal_id: Option<String>,
}

impl ApplyVoucherRequest {
    pub fn new(
        code: String,
        cart_subtotal: Decimal,
        merchant_principal_id: Option<String>,
    ) -> Self {
        return Self {
            code,
            cart_subtotal,
            merchant_principal_id,
        };
    }
}
