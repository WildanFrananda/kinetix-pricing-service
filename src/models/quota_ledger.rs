use chrono::{DateTime, Utc};
use diesel::pg::Pg;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::schema::{flash_sale_allocations, voucher_redemptions};

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Selectable, Insertable)]
#[diesel(table_name = voucher_redemptions)]
#[diesel(check_for_backend(Pg))]
pub struct VoucherRedemption {
    pub id: Uuid,
    pub voucher_code: String,
    pub order_number: String,
    pub customer_principal_id: String,
    pub released_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl VoucherRedemption {
    pub fn is_held(&self) -> bool {
        self.released_at.is_none()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Selectable, Insertable)]
#[diesel(table_name = flash_sale_allocations)]
#[diesel(check_for_backend(Pg))]
pub struct FlashSaleAllocation {
    pub id: Uuid,
    pub flash_sale_id: Uuid,
    pub product_id: String,
    pub quantity: i32,
    pub order_number: String,
    pub released_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl FlashSaleAllocation {
    pub fn is_held(&self) -> bool {
        self.released_at.is_none()
    }
}
