use rocket::serde::json::Json;
use rocket::{get, post, State};
use sqlx::PgPool;

use crate::error::AppError;
use crate::guards::AdminOrMerchantGuard;
use crate::models::{CreateVoucherRequest, Voucher};
use crate::repositories::{VoucherRepository, VoucherRepositoryPort};

#[post("/api/v1/vouchers", data = "<req>")]
pub async fn create_voucher(
    _auth: AdminOrMerchantGuard,
    pool: &State<PgPool>,
    req: Json<CreateVoucherRequest>,
) -> Result<Json<Voucher>, AppError> {
    let repo = VoucherRepository;
    let voucher = repo.create(pool.inner(), req.into_inner()).await?;
    return Ok(Json(voucher));
}

#[get("/api/v1/vouchers/<code>")]
pub async fn get_voucher(
    pool: &State<PgPool>,
    code: &str,
) -> Result<Json<Voucher>, AppError> {
    let repo = VoucherRepository;
    let voucher = repo.find_by_code(pool.inner(), code)
        .await?
        .ok_or_else(|| {
            return AppError::NotFound(format!("Voucher '{}' not found", code));
        })?;
    return Ok(Json(voucher));
}
