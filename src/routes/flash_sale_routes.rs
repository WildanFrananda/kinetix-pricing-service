use rocket::serde::json::Json;
use rocket::{get, post, State};

use crate::error::AppError;
use crate::guards::AdminOrMerchantGuard;
use crate::models::{CreateFlashSaleRequest, FlashSale};
use crate::repositories::{FlashSaleRepository, FlashSaleRepositoryPort};
use crate::DbPool;

#[post("/api/v1/flash-sales", data = "<req>")]
pub async fn create_flash_sale(
    auth: AdminOrMerchantGuard,
    pool: &State<DbPool>,
    req: Json<CreateFlashSaleRequest>,
) -> Result<Json<FlashSale>, AppError> {
    let _caller: &str = &auth.principal_id;

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
    pool: &State<DbPool>,
    product_id: &str,
) -> Result<Json<Option<FlashSale>>, AppError> {
    let repo = FlashSaleRepository;
    let flash_sale = repo.find_active_for_product(pool.inner(), product_id).await?;
    return Ok(Json(flash_sale));
}
