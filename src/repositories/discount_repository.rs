use async_trait::async_trait;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::discount::DiscountType;
use crate::models::{CreateDiscountRequest, Discount};
use crate::repositories::traits::DiscountRepositoryPort;

pub struct DiscountRepository;

#[async_trait]
impl DiscountRepositoryPort for DiscountRepository {
    async fn find_all_active(&self, pool: &PgPool) -> Result<Vec<Discount>, AppError> {
        let now = Utc::now();
        let records = sqlx::query_as::<_, Discount>(
            r#"
            SELECT id, title, discount_type, value, target_product_id, target_category_id, active, start_time, end_time, created_at, updated_at
            FROM discounts
            WHERE active = true
              AND start_time <= $1
              AND end_time >= $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(now)
        .fetch_all(pool)
        .await?;

        return Ok(records);
    }

    async fn create(&self, pool: &PgPool, req: CreateDiscountRequest) -> Result<Discount, AppError> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let discount_type_str = req.discount_type.as_str();
        let _verified_type = DiscountType::from_str(discount_type_str);

        let record = sqlx::query_as::<_, Discount>(
            r#"
            INSERT INTO discounts (id, title, discount_type, value, target_product_id, target_category_id, active, start_time, end_time, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, true, $7, $8, $9, $9)
            RETURNING id, title, discount_type, value, target_product_id, target_category_id, active, start_time, end_time, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(req.title)
        .bind(discount_type_str)
        .bind(req.value)
        .bind(req.target_product_id)
        .bind(req.target_category_id)
        .bind(req.start_time)
        .bind(req.end_time)
        .bind(now)
        .fetch_one(pool)
        .await?;

        return Ok(record);
    }
}
