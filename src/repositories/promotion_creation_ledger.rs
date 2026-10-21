use chrono::Utc;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use serde::Serialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::{IdempotentCreation, PromotionCreation, PromotionKind};
use crate::repositories::creation_claim::CreationClaim;
use crate::schema::promotion_creations::dsl as ledger;

pub struct PromotionCreationLedger;

impl PromotionCreationLedger {
    pub fn fingerprint<T: Serialize>(request: &T) -> Result<String, AppError> {
        return serde_json::to_string(request).map_err(|error| {
            return AppError::Internal(format!("the request could not be fingerprinted: {error}"));
        });
    }

    pub async fn claim(
        conn: &mut AsyncPgConnection,
        creation: &IdempotentCreation,
        kind: PromotionKind,
        promotion_id: Uuid,
        request_body: &str,
    ) -> Result<CreationClaim, AppError> {
        let inserted = diesel::insert_into(ledger::promotion_creations)
            .values(PromotionCreation {
                principal_id: creation.principal_id.clone(),
                idempotency_key: creation.idempotency_key.clone(),
                promotion_kind: kind.as_str().to_string(),
                promotion_id,
                request_body: request_body.to_string(),
                created_at: Utc::now(),
            })
            .on_conflict((ledger::principal_id, ledger::idempotency_key))
            .do_nothing()
            .execute(conn)
            .await?;

        if inserted == 1 {
            return Ok(CreationClaim::Fresh);
        }

        let earlier: PromotionCreation = ledger::promotion_creations
            .filter(ledger::principal_id.eq(&creation.principal_id))
            .filter(ledger::idempotency_key.eq(&creation.idempotency_key))
            .select(PromotionCreation::as_select())
            .first(conn)
            .await?;

        if earlier.promotion_kind != kind.as_str() || earlier.request_body != request_body {
            return Err(AppError::Unprocessable(
                "this Idempotency-Key was already used for a different request; send a new key \
                 to create another promotion"
                    .to_string(),
            ));
        }

        return Ok(CreationClaim::Replay(earlier.promotion_id));
    }
}
