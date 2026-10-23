use async_trait::async_trait;

use crate::error::AppError;
use crate::models::{CreateFlashSaleRequest, CreateOutcome, FlashSale, IdempotentCreation};
use crate::DbPool;

#[async_trait]
pub trait FlashSaleRepositoryPort: Send + Sync {
    async fn find_active_for_product(
        &self,
        pool: &DbPool,
        product_id: &str,
        units: i32,
    ) -> Result<Option<FlashSale>, AppError>;
    async fn find_active_for_cart(
        &self,
        pool: &DbPool,
        product_id: &str,
        units: i32,
        cart_merchant: Option<&str>,
    ) -> Result<Option<FlashSale>, AppError>;
    async fn earlier_creation(
        &self,
        pool: &DbPool,
        req: &CreateFlashSaleRequest,
        creation: &IdempotentCreation,
    ) -> Result<Option<FlashSale>, AppError>;
    async fn create(
        &self,
        pool: &DbPool,
        req: CreateFlashSaleRequest,
        owner: Option<String>,
        creation: IdempotentCreation,
    ) -> Result<CreateOutcome<FlashSale>, AppError>;
}
