use chrono::Utc;
use rocket::serde::json::Json;
use rocket::{get, post, State};

use crate::error::AppError;
use crate::guards::{AdminActionTarget, AdminOrMerchantGuard, IdempotencyKey, JsonBody};
use crate::models::{CreateFlashSaleRequest, FlashSale, IdempotentCreation};
use crate::repositories::{FlashSaleRepository, FlashSaleRepositoryPort};
use crate::routes::CreatedResponse;
use crate::services::{check_flash_sale, PromotionService};
use crate::DbPool;

const ONE_UNIT: i32 = 1;

#[post("/api/v1/flash-sales", data = "<req>")]
pub async fn create_flash_sale(
    auth: AdminOrMerchantGuard,
    target: AdminActionTarget,
    key: IdempotencyKey,
    pool: &State<DbPool>,
    promotions: &State<PromotionService>,
    req: JsonBody<CreateFlashSaleRequest>,
) -> Result<CreatedResponse<Json<FlashSale>>, AppError> {
    let payload = req.into_inner();
    let creation = IdempotentCreation {
        principal_id: auth.principal_id.clone(),
        idempotency_key: key.0,
    };

    let earlier = FlashSaleRepository
        .earlier_creation(pool.inner(), &payload, &creation)
        .await?;
    if let Some(record) = earlier {
        target.name(&record.id.to_string());
        return Ok(CreatedResponse::new(Json(record), true));
    }

    check_flash_sale(&payload, Utc::now())?;

    let owner = promotions
        .owner_for(&auth.principal_id, &auth.role, Some(&payload.product_id))
        .await?;

    let outcome = FlashSaleRepository
        .create(pool.inner(), payload, owner, creation)
        .await?;
    target.name(&outcome.record.id.to_string());

    return Ok(CreatedResponse::new(Json(outcome.record), outcome.replayed));
}

#[get("/api/v1/flash-sales/<product_id>")]
pub async fn get_flash_sale_for_product(
    pool: &State<DbPool>,
    product_id: &str,
) -> Result<Json<Option<FlashSale>>, AppError> {
    let repo = FlashSaleRepository;
    let flash_sale = repo
        .find_active_for_product(pool.inner(), product_id, ONE_UNIT)
        .await?;
    return Ok(Json(flash_sale));
}
