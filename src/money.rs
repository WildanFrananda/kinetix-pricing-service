use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use tonic::Status;

pub use crate::proto::common::v1::Money;

pub const CURRENCY: &str = "IDR";

const MINOR_PER_MAJOR: i64 = 100;

pub fn to_money(amount: Decimal) -> Option<Money> {
    let minor = amount
        .checked_mul(Decimal::from(MINOR_PER_MAJOR))?
        .round()
        .to_i64()?;

    return Some(Money {
        amount_minor: minor,
        currency: CURRENCY.to_string(),
    });
}

pub fn from_money(money: &Money) -> Result<Decimal, Status> {
    if !money.currency.is_empty() && money.currency != CURRENCY {
        return Err(Status::invalid_argument(format!(
            "this service prices in {} and was sent {}",
            CURRENCY, money.currency
        )));
    }

    return Ok(Decimal::from(money.amount_minor) / Decimal::from(MINOR_PER_MAJOR));
}

pub fn from_optional_money(money: &Option<Money>) -> Result<Option<Decimal>, Status> {
    return match money {
        Some(m) => from_money(m).map(Some),
        None => Ok(None),
    };
}
