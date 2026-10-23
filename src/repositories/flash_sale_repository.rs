use async_trait::async_trait;
use chrono::Utc;
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use uuid::Uuid;

use crate::error::AppError;
use crate::models::{
    CreateFlashSaleRequest, CreateOutcome, FlashSale, IdempotentCreation, PromotionKind,
};
use crate::repositories::{CreationClaim, FlashSaleRepositoryPort, PromotionCreationLedger};
use crate::schema::flash_sales::dsl::*;
use crate::DbPool;

pub struct FlashSaleRepository;

#[async_trait]
impl FlashSaleRepositoryPort for FlashSaleRepository {
    async fn find_active_for_product(
        &self,
        pool: &DbPool,
        target_product_id: &str,
        units: i32,
    ) -> Result<Option<FlashSale>, AppError> {
        let mut conn = pool.get().await?;

        let record = selling(target_product_id, units)
            .select(FlashSale::as_select())
            .first(&mut conn)
            .await
            .optional()?;

        return Ok(record);
    }

    async fn find_active_for_cart(
        &self,
        pool: &DbPool,
        target_product_id: &str,
        units: i32,
        cart_merchant: Option<&str>,
    ) -> Result<Option<FlashSale>, AppError> {
        let mut conn = pool.get().await?;
        let platform = merchant_principal_id.is_null();
        let scoped = match cart_merchant {
            Some(merchant) => selling(target_product_id, units)
                .filter(platform.or(merchant_principal_id.eq(merchant))),
            None => selling(target_product_id, units).filter(platform),
        };

        let record = scoped
            .select(FlashSale::as_select())
            .first(&mut conn)
            .await
            .optional()?;

        return Ok(record);
    }

    async fn earlier_creation(
        &self,
        pool: &DbPool,
        req: &CreateFlashSaleRequest,
        creation: &IdempotentCreation,
    ) -> Result<Option<FlashSale>, AppError> {
        let request_body = PromotionCreationLedger::fingerprint(req)?;
        let mut conn = pool.get().await?;
        let earlier = PromotionCreationLedger::earlier(
            &mut conn,
            creation,
            PromotionKind::FlashSale,
            &request_body,
        )
        .await?;
        let Some(earlier_id) = earlier else {
            return Ok(None);
        };

        return Ok(Some(created(&mut conn, earlier_id).await?));
    }

    async fn create(
        &self,
        pool: &DbPool,
        req: CreateFlashSaleRequest,
        owner: Option<String>,
        creation: IdempotentCreation,
    ) -> Result<CreateOutcome<FlashSale>, AppError> {
        let request_body = PromotionCreationLedger::fingerprint(&req)?;
        let mut conn = pool.get().await?;

        return conn
            .transaction::<_, AppError, _>(|conn| {
                async move {
                    let new_id = Uuid::new_v4();
                    let claim = PromotionCreationLedger::claim(
                        conn,
                        &creation,
                        PromotionKind::FlashSale,
                        new_id,
                        &request_body,
                    )
                    .await?;

                    if let CreationClaim::Replay(earlier_id) = claim {
                        let record = created(conn, earlier_id).await?;
                        return Ok(CreateOutcome {
                            record,
                            replayed: true,
                        });
                    }

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
                        merchant_principal_id: owner,
                    };

                    let record = diesel::insert_into(flash_sales)
                        .values(&new_flash_sale)
                        .get_result::<FlashSale>(conn)
                        .await?;

                    return Ok(CreateOutcome {
                        record,
                        replayed: false,
                    });
                }
                .scope_boxed()
            })
            .await;
    }
}

async fn created(conn: &mut AsyncPgConnection, wanted: Uuid) -> Result<FlashSale, AppError> {
    let record = flash_sales
        .filter(id.eq(wanted))
        .select(FlashSale::as_select())
        .first(conn)
        .await?;

    return Ok(record);
}

fn selling<'a>(
    target_product_id: &'a str,
    units: i32,
) -> crate::schema::flash_sales::BoxedQuery<'a, Pg> {
    let now = Utc::now();
    let wanted = units.max(1);

    return flash_sales
        .filter(product_id.eq(target_product_id))
        .filter(active.eq(true))
        .filter(start_time.le(now))
        .filter(end_time.ge(now))
        .filter(stock_sold.le(stock_limit - wanted))
        .order((flash_price.asc(), created_at.asc(), id.asc()))
        .into_boxed();
}
