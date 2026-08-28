use chrono::{DateTime, Utc};
use diesel::pg::Pg;
use diesel::prelude::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::schema::flash_sales;

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Selectable, Insertable)]
#[diesel(table_name = flash_sales)]
#[diesel(check_for_backend(Pg))]
pub struct FlashSale {
    pub id: Uuid,
    pub title: String,
    pub product_id: String,
    pub flash_price: Decimal,
    pub stock_limit: i32,
    pub stock_sold: i32,
    pub active: bool,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateFlashSaleRequest {
    pub title: String,
    pub product_id: String,
    pub flash_price: Decimal,
    pub stock_limit: i32,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
}
