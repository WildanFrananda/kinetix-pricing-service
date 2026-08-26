use rocket::serde::json::Json;
use rocket::{get, post, State};
use sqlx::PgPool;

use crate::error::AppError;
use crate::guards::AdminOrMerchantGuard;
use crate::models::{CreateFlashSaleRequest, FlashSale};
use crate::repositories::{FlashSaleRepository, FlashSaleRepositoryPort};

#[post("/api/v1/flash-sales", data = "<req>")]
pub async fn create_flash_sale(
    _auth: AdminOrMerchantGuard,
    pool: &State<PgPool>,
    req: Json<CreateFlashSaleRequest>,
) -> Result<Json<FlashSale>, AppError> {
    let repo = FlashSaleRepository;
    let flash_sale = repo.create(pool.inner(), req.into_inner()).await?;
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
