use async_trait::async_trait;

use super::merchant_standing::MerchantStanding;
use super::peer_error::PeerError;

#[async_trait]
pub trait MerchantDirectoryPort: Send + Sync {
    async fn standing_of(&self, principal_id: &str) -> Result<Option<MerchantStanding>, PeerError>;
}
