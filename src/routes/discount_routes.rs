use rocket::serde::json::Json;
use rocket::{get, post, State};

use crate::error::AppError;
use crate::guards::AdminOrMerchantGuard;
use crate::models::{CreateDiscountRequest, Discount};
use crate::repositories::{DiscountRepository, DiscountRepositoryPort};
use crate::DbPool;

#[post("/api/v1/discounts", data = "<req>")]
pub async fn create_discount(
    auth: AdminOrMerchantGuard,
    pool: &State<DbPool>,
    req: Json<CreateDiscountRequest>,
) -> Result<Json<Discount>, AppError> {
    let _caller: &str = &auth.principal_id;

    let payload = req.into_inner();
    if payload.value <= rust_decimal_macros::dec!(0.0) {
        return Err(AppError::BadRequest("Discount value must be greater than zero".to_string()));
    }
    if payload.start_time >= payload.end_time {
        return Err(AppError::BadRequest("Discount start_time must be before end_time".to_string()));
    }

    let repo = DiscountRepository;
    let discount = repo.create(pool.inner(), payload).await?;
    return Ok(Json(discount));
}

#[get("/api/v1/discounts")]
pub async fn list_discounts(pool: &State<DbPool>) -> Result<Json<Vec<Discount>>, AppError> {
    let repo = DiscountRepository;
    let discounts = repo.find_all_active(pool.inner()).await?;
    return Ok(Json(discounts));
}
