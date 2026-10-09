use rocket::serde::json::Json;
use rocket::{get, post, State};

use crate::error::AppError;
use crate::guards::AdminOrMerchantGuard;
use crate::models::{CreateDiscountRequest, Discount};
use crate::repositories::{DiscountRepository, DiscountRepositoryPort};
use crate::services::{validate_percentage, PromotionService};
use crate::DbPool;

#[post("/api/v1/discounts", data = "<req>")]
pub async fn create_discount(
    auth: AdminOrMerchantGuard,
    pool: &State<DbPool>,
    promotions: &State<PromotionService>,
    req: Json<CreateDiscountRequest>,
) -> Result<Json<Discount>, AppError> {
    let payload = req.into_inner();
    if payload.value <= rust_decimal_macros::dec!(0.0) {
        return Err(AppError::BadRequest(
            "Discount value must be greater than zero".to_string(),
        ));
    }
    if payload.start_time >= payload.end_time {
        return Err(AppError::BadRequest(
            "Discount start_time must be before end_time".to_string(),
        ));
    }

    validate_percentage(&payload.discount_type, payload.value)?;

    let owner = promotions
        .owner_for(
            &auth.principal_id,
            &auth.role,
            payload.target_product_id.as_deref(),
        )
        .await?;

    let repo = DiscountRepository;
    let discount = repo.create(pool.inner(), payload, owner).await?;
    return Ok(Json(discount));
}

#[get("/api/v1/discounts")]
pub async fn list_discounts(pool: &State<DbPool>) -> Result<Json<Vec<Discount>>, AppError> {
    let repo = DiscountRepository;
    let discounts = repo.find_all_active(pool.inner()).await?;
    return Ok(Json(discounts));
}
