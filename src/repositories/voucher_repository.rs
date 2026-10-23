use async_trait::async_trait;
use chrono::Utc;
use diesel::prelude::*;
use diesel::result::{DatabaseErrorKind, Error as DieselError};
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use uuid::Uuid;

use crate::error::AppError;
use crate::models::{
    normalized_voucher_code, CreateOutcome, CreateVoucherRequest, IdempotentCreation,
    PromotionKind, Voucher,
};
use crate::repositories::{CreationClaim, PromotionCreationLedger, VoucherRepositoryPort};
use crate::schema::vouchers::dsl::*;
use crate::DbPool;

const UNIQUE_CODE_CONSTRAINTS: [&str; 2] = ["vouchers_code_key", "vouchers_code_upper_key"];

pub struct VoucherRepository;

#[async_trait]
impl VoucherRepositoryPort for VoucherRepository {
    async fn find_by_code(
        &self,
        pool: &DbPool,
        search_code: &str,
    ) -> Result<Option<Voucher>, AppError> {
        let mut conn = pool.get().await?;
        let now = Utc::now();
        let wanted = normalized_voucher_code(search_code);

        let record = vouchers
            .filter(code.eq(&wanted))
            .filter(active.eq(true))
            .filter(expires_at.gt(now))
            .filter(used_count.lt(quota))
            .select(Voucher::as_select())
            .first(&mut conn)
            .await
            .optional()?;

        return Ok(record);
    }

    async fn earlier_creation(
        &self,
        pool: &DbPool,
        req: &CreateVoucherRequest,
        creation: &IdempotentCreation,
    ) -> Result<Option<Voucher>, AppError> {
        let mut normalized = req.clone();
        normalized.code = normalized_voucher_code(&req.code);
        let request_body = PromotionCreationLedger::fingerprint(&normalized)?;
        let mut conn = pool.get().await?;
        let earlier = PromotionCreationLedger::earlier(
            &mut conn,
            creation,
            PromotionKind::Voucher,
            &request_body,
        )
        .await?;
        let Some(earlier_id) = earlier else {
            return Ok(None);
        };

        return Ok(Some(created(&mut conn, earlier_id).await?));
    }

    async fn create(
        &self,
        pool: &DbPool,
        req: CreateVoucherRequest,
        owner: Option<String>,
        creation: IdempotentCreation,
    ) -> Result<CreateOutcome<Voucher>, AppError> {
        let mut req = req;
        req.code = normalized_voucher_code(&req.code);
        let request_body = PromotionCreationLedger::fingerprint(&req)?;
        let mut conn = pool.get().await?;

        return conn
            .transaction::<_, AppError, _>(|conn| {
                async move {
                    let new_id = Uuid::new_v4();
                    let claim = PromotionCreationLedger::claim(
                        conn,
                        &creation,
                        PromotionKind::Voucher,
                        new_id,
                        &request_body,
                    )
                    .await?;

                    if let CreationClaim::Replay(earlier_id) = claim {
                        let record = created(conn, earlier_id).await?;
                        return Ok(CreateOutcome {
                            record,
                            replayed: true,
                        });
                    }

                    let now = Utc::now();
                    let requested_code = req.code.clone();
                    let new_voucher = Voucher {
                        id: new_id,
                        code: req.code,
                        title: req.title,
                        discount_type: req.discount_type.as_str().to_string(),
                        value: req.value,
                        min_spend: req.min_spend,
                        max_discount: req.max_discount,
                        quota: req.quota,
                        used_count: 0,
                        active: true,
                        expires_at: req.expires_at,
                        created_at: now,
                        updated_at: now,
                        merchant_principal_id: owner,
                    };

                    let record = diesel::insert_into(vouchers)
                        .values(&new_voucher)
                        .get_result::<Voucher>(conn)
                        .await
                        .map_err(|error| return code_taken(error, &requested_code))?;

                    return Ok(CreateOutcome {
                        record,
                        replayed: false,
                    });
                }
                .scope_boxed()
            })
            .await;
    }
}

async fn created(conn: &mut AsyncPgConnection, wanted: Uuid) -> Result<Voucher, AppError> {
    let record = vouchers
        .filter(id.eq(wanted))
        .select(Voucher::as_select())
        .first(conn)
        .await?;

    return Ok(record);
}

fn code_taken(error: DieselError, requested_code: &str) -> AppError {
    if let DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, info) = &error {
        if info
            .constraint_name()
            .is_some_and(|name| return UNIQUE_CODE_CONSTRAINTS.contains(&name))
        {
            return AppError::Conflict(format!(
                "a voucher with the code '{requested_code}' already exists; voucher codes are \
                 unique across every merchant"
            ));
        }
    }

    return AppError::Database(error);
}
