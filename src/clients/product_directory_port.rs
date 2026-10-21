use async_trait::async_trait;

use super::peer_error::PeerError;

#[async_trait]
pub trait ProductDirectoryPort: Send + Sync {
    async fn owner_of(&self, sku: &str) -> Result<Option<String>, PeerError>;
}
