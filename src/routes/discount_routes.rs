use chrono::Utc;
use rocket::serde::json::Json;
use rocket::{get, post, State};

use crate::error::AppError;
use crate::guards::{AdminOrMerchantGuard, IdempotencyKey, JsonBody};
use crate::models::{CreateDiscountRequest, Discount, IdempotentCreation};
use crate::repositories::{DiscountRepository, DiscountRepositoryPort};
use crate::routes::CreatedResponse;
use crate::services::{check_discount, PromotionService};
use crate::DbPool;

#[post("/api/v1/discounts", data = "<req>")]
pub async fn create_discount(
    auth: AdminOrMerchantGuard,
    key: IdempotencyKey,
    pool: &State<DbPool>,
    promotions: &State<PromotionService>,
    req: JsonBody<CreateDiscountRequest>,
) -> Result<CreatedResponse<Json<Discount>>, AppError> {
    let payload = req.into_inner();
    check_discount(&payload, Utc::now())?;

    let owner = promotions
        .owner_for(
            &auth.principal_id,
            &auth.role,
            payload.target_product_id.as_deref(),
        )
        .await?;

    let creation = IdempotentCreation {
        principal_id: auth.principal_id,
        idempotency_key: key.0,
    };
    let outcome = DiscountRepository
        .create(pool.inner(), payload, owner, creation)
        .await?;

    return Ok(CreatedResponse::new(Json(outcome.record), outcome.replayed));
}

#[get("/api/v1/discounts")]
pub async fn list_discounts(pool: &State<DbPool>) -> Result<Json<Vec<Discount>>, AppError> {
    let repo = DiscountRepository;
    let discounts = repo.find_all_active(pool.inner()).await?;
    return Ok(Json(discounts));
}
