use chrono::Utc;
use rocket::serde::json::Json;
use rocket::{get, post, State};

use crate::error::AppError;
use crate::guards::{
    AdminActionTarget, AdminOrMerchantGuard, IdempotencyKey, JsonBody, OptionalCaller,
};
use crate::models::{
    normalized_voucher_code, ApplyVoucherRequest, CreateVoucherRequest, IdempotentCreation,
    Voucher, VoucherView,
};
use crate::repositories::{VoucherRepository, VoucherRepositoryPort};
use crate::routes::CreatedResponse;
use crate::services::{check_voucher, previewed, PromotionService};
use crate::traits::DefaultVoucherEvaluator;
use crate::DbPool;

#[post("/api/v1/vouchers", data = "<req>")]
pub async fn create_voucher(
    auth: AdminOrMerchantGuard,
    target: AdminActionTarget,
    key: IdempotencyKey,
    pool: &State<DbPool>,
    promotions: &State<PromotionService>,
    req: JsonBody<CreateVoucherRequest>,
) -> Result<CreatedResponse<Json<Voucher>>, AppError> {
    let mut payload = req.into_inner();
    payload.code = normalized_voucher_code(&payload.code);
    let creation = IdempotentCreation {
        principal_id: auth.principal_id.clone(),
        idempotency_key: key.0,
    };

    let earlier = VoucherRepository
        .earlier_creation(pool.inner(), &payload, &creation)
        .await?;
    if let Some(record) = earlier {
        target.name(&record.id.to_string());
        return Ok(CreatedResponse::new(Json(record), true));
    }

    check_voucher(&payload, Utc::now())?;

    let owner = promotions
        .owner_for(&auth.principal_id, &auth.role, None)
        .await?;

    let outcome = VoucherRepository
        .create(pool.inner(), payload, owner, creation)
        .await?;
    target.name(&outcome.record.id.to_string());

    return Ok(CreatedResponse::new(Json(outcome.record), outcome.replayed));
}

#[post("/api/v1/vouchers/apply", data = "<req>")]
pub async fn apply_voucher(
    viewer: OptionalCaller,
    target: AdminActionTarget,
    pool: &State<DbPool>,
    promotions: &State<PromotionService>,
    req: JsonBody<ApplyVoucherRequest>,
) -> Result<Json<Option<VoucherView>>, AppError> {
    let payload = req.into_inner();
    let query_req = ApplyVoucherRequest::new(
        payload.code,
        payload.cart_subtotal,
        payload.merchant_principal_id,
    );

    if query_req.code.trim().is_empty() {
        return Err(AppError::BadRequest(
            "Voucher code cannot be blank".to_string(),
        ));
    }

    target.name(&normalized_voucher_code(&query_req.code));

    let repo = VoucherRepository;
    let voucher_opt = repo.find_by_code(pool.inner(), &query_req.code).await?;

    let shown = voucher_opt.and_then(|voucher| {
        return previewed(
            voucher,
            &DefaultVoucherEvaluator,
            query_req.cart_subtotal,
            query_req.merchant_principal_id.as_deref(),
        );
    });

    let Some(voucher) = shown else {
        return Ok(Json(None));
    };

    let view = promotions.voucher_view(voucher, viewer.0.as_ref()).await?;
    return Ok(Json(Some(view)));
}

#[get("/api/v1/vouchers/<code>")]
pub async fn get_voucher(
    viewer: OptionalCaller,
    target: AdminActionTarget,
    pool: &State<DbPool>,
    promotions: &State<PromotionService>,
    code: &str,
) -> Result<Json<VoucherView>, AppError> {
    target.name(&normalized_voucher_code(code));

    let repo = VoucherRepository;
    let voucher = repo
        .find_by_code(pool.inner(), code)
        .await?
        .ok_or_else(|| {
            return AppError::NotFound(format!("Voucher '{code}' not found"));
        })?;

    let view = promotions.voucher_view(voucher, viewer.0.as_ref()).await?;
    return Ok(Json(view));
}
