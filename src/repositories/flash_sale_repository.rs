use async_trait::async_trait;
use chrono::Utc;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::{CreateFlashSaleRequest, FlashSale};
use crate::repositories::traits::FlashSaleRepositoryPort;
use crate::schema::flash_sales::dsl::*;
use crate::DbPool;

pub struct FlashSaleRepository;

#[async_trait]
impl FlashSaleRepositoryPort for FlashSaleRepository {
    async fn find_active_for_product(&self, pool: &DbPool, target_product_id: &str) -> Result<Option<FlashSale>, AppError> {
        let mut conn = pool.get().await?;
        let now = Utc::now();

        let record = flash_sales
            .filter(product_id.eq(target_product_id))
            .filter(active.eq(true))
            .filter(start_time.le(now))
            .filter(end_time.ge(now))
            .select(FlashSale::as_select())
            .first(&mut conn)
            .await
            .optional()?;

        return Ok(record);
    }

    async fn create(&self, pool: &DbPool, req: CreateFlashSaleRequest) -> Result<FlashSale, AppError> {
        let mut conn = pool.get().await?;
        let new_id = Uuid::new_v4();
        let now = Utc::now();

        let new_flash_sale = FlashSale {
            id: new_id,
            title: req.title,
            product_id: req.product_id,
            flash_price: req.flash_price,
            stock_limit: req.stock_limit,
            stock_sold: 0,
            active: true,
            start_time: req.start_time,
            end_time: req.end_time,
            created_at: now,
            updated_at: now,
        };

        let record = diesel::insert_into(flash_sales)
            .values(&new_flash_sale)
            .get_result::<FlashSale>(&mut conn)
            .await?;

        return Ok(record);
    }
}
