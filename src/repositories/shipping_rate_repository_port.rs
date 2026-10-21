use async_trait::async_trait;

use crate::error::AppError;
use crate::models::ShippingRate;
use crate::DbPool;

#[async_trait]
pub trait ShippingRateRepositoryPort: Send + Sync {
    async fn find_by_tier(
        &self,
        pool: &DbPool,
        tier: &str,
    ) -> Result<Option<ShippingRate>, AppError>;
}
