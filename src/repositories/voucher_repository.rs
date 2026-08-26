use async_trait::async_trait;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::{CreateVoucherRequest, Voucher};
use crate::repositories::traits::VoucherRepositoryPort;

pub struct VoucherRepository;

#[async_trait]
impl VoucherRepositoryPort for VoucherRepository {
    async fn find_by_code(&self, pool: &PgPool, code: &str) -> Result<Option<Voucher>, AppError> {
        let record = sqlx::query_as::<_, Voucher>(
            r#"
            SELECT id, code, title, discount_type, value, min_spend, max_discount, quota, used_count, active, expires_at, created_at, updated_at
            FROM vouchers
            WHERE code = $1
            "#,
        )
        .bind(code)
        .fetch_optional(pool)
        .await?;

        return Ok(record);
    }

    async fn create(&self, pool: &PgPool, req: CreateVoucherRequest) -> Result<Voucher, AppError> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let discount_type_str = req.discount_type.to_string();

        let record = sqlx::query_as::<_, Voucher>(
            r#"
            INSERT INTO vouchers (id, code, title, discount_type, value, min_spend, max_discount, quota, used_count, active, expires_at, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 0, true, $9, $10, $10)
            RETURNING id, code, title, discount_type, value, min_spend, max_discount, quota, used_count, active, expires_at, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(req.code)
        .bind(req.title)
        .bind(discount_type_str)
        .bind(req.value)
        .bind(req.min_spend)
        .bind(req.max_discount)
        .bind(req.quota)
        .bind(req.expires_at)
        .bind(now)
        .fetch_one(pool)
        .await?;

        return Ok(record);
    }
}
