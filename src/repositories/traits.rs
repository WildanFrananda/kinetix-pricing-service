use async_trait::async_trait;

use crate::error::AppError;
use crate::models::{
    CreateDiscountRequest, CreateFlashSaleRequest, CreateVoucherRequest, Discount, FlashSale, Voucher,
};
use crate::DbPool;

#[async_trait]
pub trait DiscountRepositoryPort: Send + Sync {
    async fn find_all_active(&self, pool: &DbPool) -> Result<Vec<Discount>, AppError>;
    async fn create(&self, pool: &DbPool, req: CreateDiscountRequest) -> Result<Discount, AppError>;
}

#[async_trait]
pub trait VoucherRepositoryPort: Send + Sync {
    async fn find_by_code(&self, pool: &DbPool, code: &str) -> Result<Option<Voucher>, AppError>;
    async fn create(&self, pool: &DbPool, req: CreateVoucherRequest) -> Result<Voucher, AppError>;
}

#[async_trait]
pub trait FlashSaleRepositoryPort: Send + Sync {
    async fn find_active_for_product(&self, pool: &DbPool, product_id: &str) -> Result<Option<FlashSale>, AppError>;
    async fn create(&self, pool: &DbPool, req: CreateFlashSaleRequest) -> Result<FlashSale, AppError>;
}
