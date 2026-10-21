use async_trait::async_trait;

use crate::error::AppError;
use crate::models::{CreateDiscountRequest, CreateOutcome, Discount, IdempotentCreation};
use crate::DbPool;

#[async_trait]
pub trait DiscountRepositoryPort: Send + Sync {
    async fn find_all_active(&self, pool: &DbPool) -> Result<Vec<Discount>, AppError>;
    async fn create(
        &self,
        pool: &DbPool,
        req: CreateDiscountRequest,
        owner: Option<String>,
        creation: IdempotentCreation,
    ) -> Result<CreateOutcome<Discount>, AppError>;
}
