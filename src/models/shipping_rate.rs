use chrono::{DateTime, Utc};
use diesel::pg::Pg;
use diesel::prelude::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::models::shipping_quote_request::ShippingQuoteRequest;
use crate::schema::shipping_rates;

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Selectable, Insertable)]
#[diesel(table_name = shipping_rates)]
#[diesel(check_for_backend(Pg))]
pub struct ShippingRate {
    pub service_tier: String,
    pub base_fee: Decimal,
    pub per_km_fee: Decimal,
    pub per_kg_fee: Decimal,
    pub per_kg_per_100km_fee: Decimal,
    pub currency: String,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

const GRAMS_PER_KG: Decimal = dec!(1000);
const METRES_PER_HUNDRED_KM: Decimal = dec!(100);

impl ShippingRate {
    pub fn fee_for(&self, quote: &ShippingQuoteRequest) -> Option<Decimal> {
        let weight_kg = Decimal::from(quote.total_weight_grams).checked_div(GRAMS_PER_KG)?;
        let hundred_km_units = Self::hundred_km_units(quote.distance_km)?;

        let fee = self
            .base_fee
            .checked_add(self.per_km_fee.checked_mul(quote.distance_km)?)?
            .checked_add(self.per_kg_fee.checked_mul(weight_kg)?)?
            .checked_add(
                self.per_kg_per_100km_fee
                    .checked_mul(weight_kg)?
                    .checked_mul(hundred_km_units)?,
            )?;

        return Some(fee.round_dp(2).max(dec!(0.00)));
    }

    fn hundred_km_units(distance_km: Decimal) -> Option<Decimal> {
        let bands = distance_km.checked_div(METRES_PER_HUNDRED_KM)?.round_dp(1);

        return Some(bands.max(dec!(1.0)));
    }
}
