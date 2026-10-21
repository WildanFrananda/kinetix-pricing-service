use async_trait::async_trait;
use chrono::Utc;
use diesel::prelude::*;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, RunQueryDsl};
use uuid::Uuid;

use crate::error::AppError;
use crate::models::{
    CreateDiscountRequest, CreateOutcome, Discount, IdempotentCreation, PromotionKind,
};
use crate::repositories::{CreationClaim, DiscountRepositoryPort, PromotionCreationLedger};
use crate::schema::discounts::dsl::*;
use crate::DbPool;

pub struct DiscountRepository;

#[async_trait]
impl DiscountRepositoryPort for DiscountRepository {
    async fn find_all_active(&self, pool: &DbPool) -> Result<Vec<Discount>, AppError> {
        let mut conn = pool.get().await?;
        let now = Utc::now();
        let records = discounts
            .filter(active.eq(true))
            .filter(start_time.le(now))
            .filter(end_time.ge(now))
            .order(created_at.desc())
            .select(Discount::as_select())
            .load(&mut conn)
            .await?;

        return Ok(records);
    }

    async fn create(
        &self,
        pool: &DbPool,
        req: CreateDiscountRequest,
        owner: Option<String>,
        creation: IdempotentCreation,
    ) -> Result<CreateOutcome<Discount>, AppError> {
        let request_body = PromotionCreationLedger::fingerprint(&req)?;
        let mut conn = pool.get().await?;

        return conn
            .transaction::<_, AppError, _>(|conn| {
                async move {
                    let new_id = Uuid::new_v4();
                    let claim = PromotionCreationLedger::claim(
                        conn,
                        &creation,
                        PromotionKind::Discount,
                        new_id,
                        &request_body,
                    )
                    .await?;

                    if let CreationClaim::Replay(earlier_id) = claim {
                        let record = discounts
                            .filter(id.eq(earlier_id))
                            .select(Discount::as_select())
                            .first(conn)
                            .await?;
                        return Ok(CreateOutcome {
                            record,
                            replayed: true,
                        });
                    }

                    let now = Utc::now();
                    let new_discount = Discount {
                        id: new_id,
                        title: req.title,
                        discount_type: req.discount_type.as_str().to_string(),
                        value: req.value,
                        target_product_id: req.target_product_id,
                        target_category_id: req.target_category_id,
                        active: true,
                        start_time: req.start_time,
                        end_time: req.end_time,
                        created_at: now,
                        updated_at: now,
                        merchant_principal_id: owner,
                    };

                    let record = diesel::insert_into(discounts)
                        .values(&new_discount)
                        .get_result::<Discount>(conn)
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
