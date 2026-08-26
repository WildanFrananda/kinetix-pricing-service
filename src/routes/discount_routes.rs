use rocket::serde::json::Json;
use rocket::{get, post, State};
use sqlx::PgPool;

use crate::error::AppError;
use crate::guards::AdminOrMerchantGuard;
use crate::models::{CreateDiscountRequest, Discount};
use crate::repositories::{DiscountRepository, DiscountRepositoryPort};

#[post("/api/v1/discounts", data = "<req>")]
pub async fn create_discount(
    _auth: AdminOrMerchantGuard,
    pool: &State<PgPool>,
    req: Json<CreateDiscountRequest>,
) -> Result<Json<Discount>, AppError> {
    let repo = DiscountRepository;
    let discount = repo.create(pool.inner(), req.into_inner()).await?;
    return Ok(Json(discount));
}

#[get("/api/v1/discounts")]
pub async fn list_discounts(pool: &State<PgPool>) -> Result<Json<Vec<Discount>>, AppError> {
    let repo = DiscountRepository;
    let discounts = repo.find_all_active(pool.inner()).await?;
    return Ok(Json(discounts));
}
