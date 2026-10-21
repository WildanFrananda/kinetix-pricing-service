#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MerchantStanding {
    pub merchant_principal_id: String,
    pub may_sell: bool,
}
