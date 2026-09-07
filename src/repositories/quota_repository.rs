use chrono::Utc;
use diesel::prelude::*;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, RunQueryDsl};
use uuid::Uuid;

use crate::error::AppError;
use crate::models::{FlashSaleAllocation, VoucherRedemption};
use crate::DbPool;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuotaOutcome {
    Applied { remaining: i32 },
    AlreadyDone { remaining: i32 },
    Exhausted,
    NotFound,
}

pub struct QuotaRepository;

impl QuotaRepository {
    pub async fn redeem_voucher(
        pool: &DbPool,
        code_value: &str,
        order: &str,
        customer: &str,
    ) -> Result<QuotaOutcome, AppError> {
        use crate::schema::voucher_redemptions::dsl as ledger;
        use crate::schema::vouchers::dsl as v;

        let mut conn = pool.get().await?;
        let code_owned = code_value.to_string();
        let order_owned = order.to_string();
        let customer_owned = customer.to_string();

        conn.transaction::<_, AppError, _>(|conn| {
            async move {
                let voucher: Option<(i32, i32)> = v::vouchers
                    .filter(v::code.eq(&code_owned))
                    .select((v::quota, v::used_count))
                    .first(conn)
                    .await
                    .optional()?;

                let Some((quota_value, _)) = voucher else {
                    return Ok(QuotaOutcome::NotFound);
                };

                let existing: Option<VoucherRedemption> = ledger::voucher_redemptions
                    .filter(ledger::voucher_code.eq(&code_owned))
                    .filter(ledger::order_number.eq(&order_owned))
                    .select(VoucherRedemption::as_select())
                    .first(conn)
                    .await
                    .optional()?;

                if let Some(row) = &existing {
                    if row.is_held() {
                        let used: i32 = v::vouchers
                            .filter(v::code.eq(&code_owned))
                            .select(v::used_count)
                            .first(conn)
                            .await?;
                        return Ok(QuotaOutcome::AlreadyDone {
                            remaining: quota_value - used,
                        });
                    }
                }

                let used_after: Option<i32> = diesel::update(
                    v::vouchers
                        .filter(v::code.eq(&code_owned))
                        .filter(v::active.eq(true))
                        .filter(v::used_count.lt(v::quota)),
                )
                .set((
                    v::used_count.eq(v::used_count + 1),
                    v::updated_at.eq(Utc::now()),
                ))
                .returning(v::used_count)
                .get_result(conn)
                .await
                .optional()?;

                let Some(used_after) = used_after else {
                    return Ok(QuotaOutcome::Exhausted);
                };

                match existing {
                    Some(row) => {
                        diesel::update(ledger::voucher_redemptions.filter(ledger::id.eq(row.id)))
                            .set(ledger::released_at.eq(None::<chrono::DateTime<Utc>>))
                            .execute(conn)
                            .await?;
                    }
                    None => {
                        diesel::insert_into(ledger::voucher_redemptions)
                            .values(VoucherRedemption {
                                id: Uuid::new_v4(),
                                voucher_code: code_owned.clone(),
                                order_number: order_owned.clone(),
                                customer_principal_id: customer_owned,
                                released_at: None,
                                created_at: Utc::now(),
                            })
                            .execute(conn)
                            .await?;
                    }
                }

                Ok(QuotaOutcome::Applied {
                    remaining: quota_value - used_after,
                })
            }
            .scope_boxed()
        })
        .await
    }

    pub async fn release_voucher(
        pool: &DbPool,
        code_value: &str,
        order: &str,
    ) -> Result<QuotaOutcome, AppError> {
        use crate::schema::voucher_redemptions::dsl as ledger;
        use crate::schema::vouchers::dsl as v;

        let mut conn = pool.get().await?;
        let code_owned = code_value.to_string();
        let order_owned = order.to_string();

        conn.transaction::<_, AppError, _>(|conn| {
            async move {
                let voucher: Option<(i32, i32)> = v::vouchers
                    .filter(v::code.eq(&code_owned))
                    .select((v::quota, v::used_count))
                    .first(conn)
                    .await
                    .optional()?;

                let Some((quota_value, used_now)) = voucher else {
                    return Ok(QuotaOutcome::NotFound);
                };

                let existing: Option<VoucherRedemption> = ledger::voucher_redemptions
                    .filter(ledger::voucher_code.eq(&code_owned))
                    .filter(ledger::order_number.eq(&order_owned))
                    .select(VoucherRedemption::as_select())
                    .first(conn)
                    .await
                    .optional()?;

                let Some(row) = existing else {
                    return Ok(QuotaOutcome::AlreadyDone {
                        remaining: quota_value - used_now,
                    });
                };

                if !row.is_held() {
                    return Ok(QuotaOutcome::AlreadyDone {
                        remaining: quota_value - used_now,
                    });
                }

                diesel::update(ledger::voucher_redemptions.filter(ledger::id.eq(row.id)))
                    .set(ledger::released_at.eq(Some(Utc::now())))
                    .execute(conn)
                    .await?;

                let used_after: i32 = diesel::update(
                    v::vouchers
                        .filter(v::code.eq(&code_owned))
                        .filter(v::used_count.gt(0)),
                )
                .set((
                    v::used_count.eq(v::used_count - 1),
                    v::updated_at.eq(Utc::now()),
                ))
                .returning(v::used_count)
                .get_result(conn)
                .await
                .optional()?
                .unwrap_or(0);

                Ok(QuotaOutcome::Applied {
                    remaining: quota_value - used_after,
                })
            }
            .scope_boxed()
        })
        .await
    }

    pub async fn allocate_flash_sale(
        pool: &DbPool,
        sale_id: Uuid,
        product: &str,
        quantity_wanted: i32,
        order: &str,
    ) -> Result<QuotaOutcome, AppError> {
        use crate::schema::flash_sale_allocations::dsl as ledger;
        use crate::schema::flash_sales::dsl as f;

        if quantity_wanted <= 0 {
            return Ok(QuotaOutcome::Exhausted);
        }

        let mut conn = pool.get().await?;
        let product_owned = product.to_string();
        let order_owned = order.to_string();

        conn.transaction::<_, AppError, _>(|conn| {
            async move {
                let sale: Option<(i32, i32)> = f::flash_sales
                    .filter(f::id.eq(sale_id))
                    .select((f::stock_limit, f::stock_sold))
                    .first(conn)
                    .await
                    .optional()?;

                let Some((limit, _)) = sale else {
                    return Ok(QuotaOutcome::NotFound);
                };

                let existing: Option<FlashSaleAllocation> = ledger::flash_sale_allocations
                    .filter(ledger::flash_sale_id.eq(sale_id))
                    .filter(ledger::order_number.eq(&order_owned))
                    .select(FlashSaleAllocation::as_select())
                    .first(conn)
                    .await
                    .optional()?;

                if let Some(row) = &existing {
                    if row.is_held() {
                        let sold: i32 = f::flash_sales
                            .filter(f::id.eq(sale_id))
                            .select(f::stock_sold)
                            .first(conn)
                            .await?;
                        return Ok(QuotaOutcome::AlreadyDone {
                            remaining: limit - sold,
                        });
                    }
                }

                let sold_after: Option<i32> = diesel::update(
                    f::flash_sales
                        .filter(f::id.eq(sale_id))
                        .filter(f::active.eq(true))
                        .filter(f::stock_sold.le(f::stock_limit - quantity_wanted)),
                )
                .set((
                    f::stock_sold.eq(f::stock_sold + quantity_wanted),
                    f::updated_at.eq(Utc::now()),
                ))
                .returning(f::stock_sold)
                .get_result(conn)
                .await
                .optional()?;

                let Some(sold_after) = sold_after else {
                    return Ok(QuotaOutcome::Exhausted);
                };

                match existing {
                    Some(row) => {
                        diesel::update(ledger::flash_sale_allocations.filter(ledger::id.eq(row.id)))
                            .set((
                                ledger::released_at.eq(None::<chrono::DateTime<Utc>>),
                                ledger::quantity.eq(quantity_wanted),
                            ))
                            .execute(conn)
                            .await?;
                    }
                    None => {
                        diesel::insert_into(ledger::flash_sale_allocations)
                            .values(FlashSaleAllocation {
                                id: Uuid::new_v4(),
                                flash_sale_id: sale_id,
                                product_id: product_owned,
                                quantity: quantity_wanted,
                                order_number: order_owned.clone(),
                                released_at: None,
                                created_at: Utc::now(),
                            })
                            .execute(conn)
                            .await?;
                    }
                }

                Ok(QuotaOutcome::Applied {
                    remaining: limit - sold_after,
                })
            }
            .scope_boxed()
        })
        .await
    }

    pub async fn release_flash_sale(
        pool: &DbPool,
        sale_id: Uuid,
        order: &str,
    ) -> Result<QuotaOutcome, AppError> {
        use crate::schema::flash_sale_allocations::dsl as ledger;
        use crate::schema::flash_sales::dsl as f;

        let mut conn = pool.get().await?;
        let order_owned = order.to_string();

        conn.transaction::<_, AppError, _>(|conn| {
            async move {
                let sale: Option<(i32, i32)> = f::flash_sales
                    .filter(f::id.eq(sale_id))
                    .select((f::stock_limit, f::stock_sold))
                    .first(conn)
                    .await
                    .optional()?;

                let Some((limit, sold_now)) = sale else {
                    return Ok(QuotaOutcome::NotFound);
                };

                let existing: Option<FlashSaleAllocation> = ledger::flash_sale_allocations
                    .filter(ledger::flash_sale_id.eq(sale_id))
                    .filter(ledger::order_number.eq(&order_owned))
                    .select(FlashSaleAllocation::as_select())
                    .first(conn)
                    .await
                    .optional()?;

                let Some(row) = existing else {
                    return Ok(QuotaOutcome::AlreadyDone {
                        remaining: limit - sold_now,
                    });
                };

                if !row.is_held() {
                    return Ok(QuotaOutcome::AlreadyDone {
                        remaining: limit - sold_now,
                    });
                }

                diesel::update(ledger::flash_sale_allocations.filter(ledger::id.eq(row.id)))
                    .set(ledger::released_at.eq(Some(Utc::now())))
                    .execute(conn)
                    .await?;

                let sold_after: i32 = diesel::update(
                    f::flash_sales
                        .filter(f::id.eq(sale_id))
                        .filter(f::stock_sold.ge(row.quantity)),
                )
                .set((
                    f::stock_sold.eq(f::stock_sold - row.quantity),
                    f::updated_at.eq(Utc::now()),
                ))
                .returning(f::stock_sold)
                .get_result(conn)
                .await
                .optional()?
                .unwrap_or(0);

                Ok(QuotaOutcome::Applied {
                    remaining: limit - sold_after,
                })
            }
            .scope_boxed()
        })
        .await
    }
}
