use chrono::{DateTime, Utc};
use diesel::pg::Pg;
use diesel::prelude::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotedShipping {
    pub service_tier: String,
    pub base_shipping_fee: Decimal,
    pub priced: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShippingQuoteRequest {
    pub service_tier: String,
    pub distance_km: Decimal,
    pub total_weight_grams: i64,
}

const GRAMS_PER_KG: Decimal = dec!(1000);
const METRES_PER_HUNDRED_KM: Decimal = dec!(100);

impl ShippingRate {
    pub fn fee_for(&self, quote: &ShippingQuoteRequest) -> Decimal {
        let weight_kg = Decimal::from(quote.total_weight_grams) / GRAMS_PER_KG;
        let hundred_km_units = Self::hundred_km_units(quote.distance_km);

        let fee = self.base_fee
            + self.per_km_fee * quote.distance_km
            + self.per_kg_fee * weight_kg
            + self.per_kg_per_100km_fee * weight_kg * hundred_km_units;

        return fee.round_dp(2).max(dec!(0.00));
    }

    fn hundred_km_units(distance_km: Decimal) -> Decimal {
        let bands = (distance_km / METRES_PER_HUNDRED_KM).round_dp(1);

        return bands.max(dec!(1.0));
    }
}
