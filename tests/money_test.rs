use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use kinetix_pricing_service::models::{ShippingQuoteRequest, ShippingRate};
use kinetix_pricing_service::money::to_money;

#[test]
fn the_largest_amount_money_carries_converts_and_one_cent_more_does_not() {
    let largest = dec!(92_233_720_368_547_758.07);

    assert_eq!(
        to_money(largest).map(|money| return money.amount_minor),
        Some(i64::MAX)
    );
    assert!(to_money(largest + dec!(0.01)).is_none());
    assert!(to_money(Decimal::MAX).is_none());
    assert!(to_money(Decimal::MIN).is_none());
}

#[test]
fn an_ordinary_amount_converts_to_minor_units() {
    let money = to_money(dec!(49_920.50)).expect("an ordinary amount converts");

    assert_eq!(money.amount_minor, 4_992_050);
    assert_eq!(money.currency, "IDR");
}

fn regular() -> ShippingRate {
    let now = chrono::Utc::now();
    return ShippingRate {
        service_tier: "KINETIX_REGULAR".to_string(),
        base_fee: dec!(9000),
        per_km_fee: dec!(3000),
        per_kg_fee: dec!(1000),
        per_kg_per_100km_fee: dec!(1500),
        currency: "IDR".to_string(),
        active: true,
        created_at: now,
        updated_at: now,
    };
}

#[test]
fn a_journey_too_long_or_heavy_to_compute_has_no_fee_rather_than_a_panic() {
    let journey = ShippingQuoteRequest {
        service_tier: "KINETIX_REGULAR".to_string(),
        distance_km: dec!(10_000_000_000_000_000_000_000),
        total_weight_grams: i64::MAX,
    };

    assert!(regular().fee_for(&journey).is_none());
}

#[test]
fn an_ordinary_journey_still_has_its_fee() {
    let journey = ShippingQuoteRequest {
        service_tier: "KINETIX_REGULAR".to_string(),
        distance_km: dec!(250),
        total_weight_grams: 4_000,
    };

    assert_eq!(regular().fee_for(&journey), Some(dec!(778000.00)));
}
