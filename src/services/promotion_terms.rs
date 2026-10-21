use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use crate::error::AppError;
use crate::models::{
    CreateDiscountRequest, CreateFlashSaleRequest, CreateVoucherRequest, DiscountType,
};

pub const TITLE_MAX_CHARS: usize = 255;
pub const IDENTIFIER_MAX_CHARS: usize = 100;
pub const MONEY_SCALE: u32 = 2;
pub const MONEY_LIMIT: Decimal = dec!(10_000_000_000);
pub const EARLIEST_STORABLE_UNIX_SECONDS: i64 = -210_866_803_200;

pub fn check_voucher(req: &CreateVoucherRequest, now: DateTime<Utc>) -> Result<(), AppError> {
    identifier("code", &req.code)?;
    if req.code.trim() != req.code {
        return Err(refused("code cannot start or end with whitespace"));
    }

    text("title", &req.title, TITLE_MAX_CHARS)?;
    positive_money("value", req.value)?;
    validate_percentage(&req.discount_type, req.value)?;

    money("min_spend", req.min_spend)?;
    if req.min_spend < Decimal::ZERO {
        return Err(refused("min_spend cannot be negative"));
    }

    if let Some(max_discount) = req.max_discount {
        positive_money("max_discount", max_discount)?;
    }

    if req.quota < 1 {
        return Err(refused("quota must be at least 1"));
    }

    if req.expires_at <= now {
        return Err(refused("expires_at must be in the future"));
    }

    return Ok(());
}

pub fn check_discount(req: &CreateDiscountRequest, now: DateTime<Utc>) -> Result<(), AppError> {
    text("title", &req.title, TITLE_MAX_CHARS)?;
    positive_money("value", req.value)?;
    validate_percentage(&req.discount_type, req.value)?;

    if let Some(product) = &req.target_product_id {
        identifier("target_product_id", product)?;
    }
    if let Some(category) = &req.target_category_id {
        identifier("target_category_id", category)?;
    }

    return window(req.start_time, req.end_time, now);
}

pub fn check_flash_sale(req: &CreateFlashSaleRequest, now: DateTime<Utc>) -> Result<(), AppError> {
    text("title", &req.title, TITLE_MAX_CHARS)?;
    identifier("product_id", &req.product_id)?;
    positive_money("flash_price", req.flash_price)?;

    if req.stock_limit < 1 {
        return Err(refused("stock_limit must be at least 1"));
    }

    return window(req.start_time, req.end_time, now);
}

pub fn validate_percentage(kind: &DiscountType, value: Decimal) -> Result<(), AppError> {
    if *kind == DiscountType::Percentage && value > dec!(100) {
        return Err(AppError::BadRequest(
            "a percentage cannot be more than 100".to_string(),
        ));
    }
    return Ok(());
}

fn window(
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    if start_time.timestamp() < EARLIEST_STORABLE_UNIX_SECONDS {
        return Err(refused(
            "start_time is earlier than any time this service can store",
        ));
    }
    if start_time >= end_time {
        return Err(refused("start_time must be before end_time"));
    }
    if end_time <= now {
        return Err(refused("end_time must be in the future"));
    }

    return Ok(());
}

fn text(field: &str, value: &str, max_chars: usize) -> Result<(), AppError> {
    if value.trim().is_empty() {
        return Err(refused(&format!("{field} cannot be blank")));
    }
    if value.chars().count() > max_chars {
        return Err(refused(&format!(
            "{field} is at most {max_chars} characters"
        )));
    }
    if value.contains('\0') {
        return Err(refused(&format!("{field} cannot contain a NUL character")));
    }

    return Ok(());
}

fn identifier(field: &str, value: &str) -> Result<(), AppError> {
    return text(field, value, IDENTIFIER_MAX_CHARS);
}

fn money(field: &str, value: Decimal) -> Result<(), AppError> {
    if value.normalize().scale() > MONEY_SCALE {
        return Err(refused(&format!(
            "{field} has more than {MONEY_SCALE} decimal places; amounts are kept to the cent and \
             this service does not round them"
        )));
    }
    if value.abs() >= MONEY_LIMIT {
        return Err(refused(&format!("{field} must be less than {MONEY_LIMIT}")));
    }

    return Ok(());
}

fn positive_money(field: &str, value: Decimal) -> Result<(), AppError> {
    money(field, value)?;
    if value <= Decimal::ZERO {
        return Err(refused(&format!("{field} must be greater than zero")));
    }

    return Ok(());
}

fn refused(message: &str) -> AppError {
    return AppError::BadRequest(message.to_string());
}
