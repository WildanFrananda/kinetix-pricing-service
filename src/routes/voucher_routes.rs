use rocket::serde::json::Json;
use rocket::{get, post, State};

use crate::error::AppError;
use crate::guards::AdminOrMerchantGuard;
use crate::models::{ApplyVoucherRequest, CreateVoucherRequest, Voucher};
use crate::repositories::{VoucherRepository, VoucherRepositoryPort};
use crate::traits::{DefaultVoucherEvaluator, VoucherEvaluator};
use crate::DbPool;

#[post("/api/v1/vouchers", data = "<req>")]
pub async fn create_voucher(
    auth: AdminOrMerchantGuard,
    pool: &State<DbPool>,
    req: Json<CreateVoucherRequest>,
) -> Result<Json<Voucher>, AppError> {
    let _caller: &str = &auth.principal_id;

    let payload = req.into_inner();
    if payload.code.trim().is_empty() {
        return Err(AppError::BadRequest("Voucher code cannot be blank".to_string()));
    }
    if payload.value <= rust_decimal_macros::dec!(0.0) {
        return Err(AppError::BadRequest("Voucher value must be greater than zero".to_string()));
    }

    let repo = VoucherRepository;
    let voucher = repo.create(pool.inner(), payload).await?;
    return Ok(Json(voucher));
}

#[post("/api/v1/vouchers/apply", data = "<req>")]
pub async fn apply_voucher(
    pool: &State<DbPool>,
    req: Json<ApplyVoucherRequest>,
) -> Result<Json<Option<Voucher>>, AppError> {
    let payload = req.into_inner();
    let query_req = ApplyVoucherRequest::new(payload.code, payload.cart_subtotal);

    if query_req.code.trim().is_empty() {
        return Err(AppError::BadRequest("Voucher code cannot be blank".to_string()));
    }

    let repo = VoucherRepository;
    let voucher_opt = repo.find_by_code(pool.inner(), &query_req.code).await?;

    if let Some(ref voucher) = voucher_opt {
        let evaluator = DefaultVoucherEvaluator;
        if !evaluator.is_eligible(voucher, query_req.cart_subtotal) {
            return Ok(Json(None));
        }
    }

    return Ok(Json(voucher_opt));
}

#[get("/api/v1/vouchers/<code>")]
pub async fn get_voucher(
    pool: &State<DbPool>,
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
