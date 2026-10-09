use async_trait::async_trait;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MerchantStanding {
    pub merchant_principal_id: String,
    pub may_sell: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerUnavailable(pub String);

#[async_trait]
pub trait MerchantDirectoryPort: Send + Sync {
    async fn standing_of(
        &self,
        principal_id: &str,
    ) -> Result<Option<MerchantStanding>, PeerUnavailable>;
}

#[async_trait]
pub trait ProductDirectoryPort: Send + Sync {
    async fn owner_of(&self, sku: &str) -> Result<Option<String>, PeerUnavailable>;
}
