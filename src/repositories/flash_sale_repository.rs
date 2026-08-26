use async_trait::async_trait;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::{CreateFlashSaleRequest, FlashSale};
use crate::repositories::traits::FlashSaleRepositoryPort;

pub struct FlashSaleRepository;

#[async_trait]
impl FlashSaleRepositoryPort for FlashSaleRepository {
    async fn find_active_for_product(&self, pool: &PgPool, product_id: &str) -> Result<Option<FlashSale>, AppError> {
        let now = Utc::now();
        let record = sqlx::query_as::<_, FlashSale>(
            r#"
            SELECT id, title, product_id, flash_price, stock_limit, stock_sold, active, start_time, end_time, created_at, updated_at
            FROM flash_sales
            WHERE product_id = $1
              AND active = true
              AND stock_sold < stock_limit
              AND start_time <= $2
              AND end_time >= $2
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(product_id)
        .bind(now)
        .fetch_optional(pool)
        .await?;

        return Ok(record);
    }

    async fn create(&self, pool: &PgPool, req: CreateFlashSaleRequest) -> Result<FlashSale, AppError> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        let record = sqlx::query_as::<_, FlashSale>(
            r#"
            INSERT INTO flash_sales (id, title, product_id, flash_price, stock_limit, stock_sold, active, start_time, end_time, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, 0, true, $6, $7, $8, $8)
            RETURNING id, title, product_id, flash_price, stock_limit, stock_sold, active, start_time, end_time, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(req.title)
        .bind(req.product_id)
        .bind(req.flash_price)
        .bind(req.stock_limit)
        .bind(req.start_time)
        .bind(req.end_time)
        .bind(now)
        .fetch_one(pool)
        .await?;

        return Ok(record);
    }
}
