use async_trait::async_trait;
use chrono::Utc;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::{CreateVoucherRequest, Voucher};
use crate::repositories::traits::VoucherRepositoryPort;
use crate::schema::vouchers::dsl::*;
use crate::DbPool;

pub struct VoucherRepository;

#[async_trait]
impl VoucherRepositoryPort for VoucherRepository {
    async fn find_by_code(&self, pool: &DbPool, search_code: &str) -> Result<Option<Voucher>, AppError> {
        let mut conn = pool.get().await?;
        let now = Utc::now();

        let record = vouchers
            .filter(code.eq(search_code))
            .filter(active.eq(true))
            .filter(expires_at.gt(now))
            .select(Voucher::as_select())
            .first(&mut conn)
            .await
            .optional()?;

        return Ok(record);
    }

    async fn create(&self, pool: &DbPool, req: CreateVoucherRequest) -> Result<Voucher, AppError> {
        let mut conn = pool.get().await?;
        let new_id = Uuid::new_v4();
        let now = Utc::now();

        let new_voucher = Voucher {
            id: new_id,
            code: req.code,
            title: req.title,
            discount_type: req.discount_type.as_str().to_string(),
            value: req.value,
            min_spend: req.min_spend,
            max_discount: req.max_discount,
            quota: req.quota,
            used_count: 0,
            active: true,
            expires_at: req.expires_at,
            created_at: now,
            updated_at: now,
        };

        let record = diesel::insert_into(vouchers)
            .values(&new_voucher)
            .get_result::<Voucher>(&mut conn)
            .await?;

        return Ok(record);
    }
}
