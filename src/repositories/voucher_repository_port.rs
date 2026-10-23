use async_trait::async_trait;

use crate::error::AppError;
use crate::models::{CreateOutcome, CreateVoucherRequest, IdempotentCreation, Voucher};
use crate::DbPool;

#[async_trait]
pub trait VoucherRepositoryPort: Send + Sync {
    async fn find_by_code(&self, pool: &DbPool, code: &str) -> Result<Option<Voucher>, AppError>;
    async fn earlier_creation(
        &self,
        pool: &DbPool,
        req: &CreateVoucherRequest,
        creation: &IdempotentCreation,
    ) -> Result<Option<Voucher>, AppError>;
    async fn create(
        &self,
        pool: &DbPool,
        req: CreateVoucherRequest,
        owner: Option<String>,
        creation: IdempotentCreation,
    ) -> Result<CreateOutcome<Voucher>, AppError>;
}
