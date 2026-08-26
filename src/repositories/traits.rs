use async_trait::async_trait;
use sqlx::PgPool;

use crate::error::AppError;
use crate::models::{
    CreateDiscountRequest, CreateFlashSaleRequest, CreateVoucherRequest, Discount, FlashSale, Voucher,
};

#[async_trait]
pub trait DiscountRepositoryPort: Send + Sync {
    async fn find_all_active(&self, pool: &PgPool) -> Result<Vec<Discount>, AppError>;
    async fn create(&self, pool: &PgPool, req: CreateDiscountRequest) -> Result<Discount, AppError>;
}

#[async_trait]
pub trait VoucherRepositoryPort: Send + Sync {
    async fn find_by_code(&self, pool: &PgPool, code: &str) -> Result<Option<Voucher>, AppError>;
    async fn create(&self, pool: &PgPool, req: CreateVoucherRequest) -> Result<Voucher, AppError>;
}

#[async_trait]
pub trait FlashSaleRepositoryPort: Send + Sync {
    async fn find_active_for_product(&self, pool: &PgPool, product_id: &str) -> Result<Option<FlashSale>, AppError>;
    async fn create(&self, pool: &PgPool, req: CreateFlashSaleRequest) -> Result<FlashSale, AppError>;
}
