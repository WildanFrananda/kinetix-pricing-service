use rocket::serde::json::Json;
use rocket::{get, post, State};
use sqlx::PgPool;

use crate::error::AppError;
use crate::guards::AdminOrMerchantGuard;
use crate::models::{ApplyVoucherRequest, CreateVoucherRequest, Voucher};
use crate::repositories::{VoucherRepository, VoucherRepositoryPort};

#[post("/api/v1/vouchers", data = "<req>")]
pub async fn create_voucher(
    auth: AdminOrMerchantGuard,
    pool: &State<PgPool>,
    req: Json<CreateVoucherRequest>,
) -> Result<Json<Voucher>, AppError> {
    println!("Create voucher request authorized for role: {}, user_id: {:?}", auth.role, auth.user_id);
    let repo = VoucherRepository;
    let voucher = repo.create(pool.inner(), req.into_inner()).await?;
    return Ok(Json(voucher));
}

#[post("/api/v1/vouchers/apply", data = "<req>")]
pub async fn apply_voucher(
    pool: &State<PgPool>,
    req: Json<ApplyVoucherRequest>,
) -> Result<Json<Option<Voucher>>, AppError> {
    let payload = req.into_inner();
    let query_req = ApplyVoucherRequest::new(payload.code, payload.cart_subtotal);
    let repo = VoucherRepository;
    let voucher = repo.find_by_code(pool.inner(), &query_req.code).await?;
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
