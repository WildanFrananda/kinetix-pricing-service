use chrono::{DateTime, Utc};
use diesel::pg::Pg;
use diesel::prelude::*;
use uuid::Uuid;

use crate::schema::promotion_creations;

#[derive(Debug, Clone, Queryable, Selectable, Insertable)]
#[diesel(table_name = promotion_creations)]
#[diesel(check_for_backend(Pg))]
pub struct PromotionCreation {
    pub principal_id: String,
    pub idempotency_key: String,
    pub promotion_kind: String,
    pub promotion_id: Uuid,
    pub request_body: String,
    pub created_at: DateTime<Utc>,
}
