use rocket::serde::json::Json;
use rocket::{get, post, State};
use sqlx::PgPool;

use crate::error::AppError;
use crate::guards::AdminOrMerchantGuard;
use crate::models::{CreateFlashSaleRequest, FlashSale};
use crate::repositories::{FlashSaleRepository, FlashSaleRepositoryPort};

#[post("/api/v1/flash-sales", data = "<req>")]
pub async fn create_flash_sale(
    auth: AdminOrMerchantGuard,
    pool: &State<PgPool>,
    req: Json<CreateFlashSaleRequest>,
) -> Result<Json<FlashSale>, AppError> {
    if auth.role == "MERCHANT" && auth.user_id.is_none() {
        return Err(AppError::Unauthorized);
    }

    let payload = req.into_inner();
    if payload.flash_price <= rust_decimal_macros::dec!(0.0) {
        return Err(AppError::BadRequest("Flash sale price must be greater than zero".to_string()));
    }
    if payload.stock_limit <= 0 {
        return Err(AppError::BadRequest("Flash sale stock limit must be positive".to_string()));
    }

    let repo = FlashSaleRepository;
    let flash_sale = repo.create(pool.inner(), payload).await?;
    return Ok(Json(flash_sale));
}

#[get("/api/v1/flash-sales/<product_id>")]
pub async fn get_flash_sale_for_product(
    pool: &State<PgPool>,
    product_id: &str,
) -> Result<Json<Option<FlashSale>>, AppError> {
    let repo = FlashSaleRepository;
    let flash_sale = repo.find_active_for_product(pool.inner(), product_id).await?;
    return Ok(Json(flash_sale));
}
