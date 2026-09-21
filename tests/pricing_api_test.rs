use async_trait::async_trait;
use chrono::{Duration, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use uuid::Uuid;

use kinetix_pricing_service::db::create_pool;
use kinetix_pricing_service::error::AppError;
use kinetix_pricing_service::models::{
    CalculatePriceRequest, CreateDiscountRequest, CreateFlashSaleRequest, CreateVoucherRequest,
    Discount, FlashSale, PriceItemRequest, ShippingQuoteRequest, ShippingRate, Voucher,
};
use kinetix_pricing_service::repositories::{
    DiscountRepositoryPort, FlashSaleRepositoryPort, ShippingRateRepositoryPort,
    VoucherRepositoryPort,
};
use kinetix_pricing_service::services::PricingService;
use kinetix_pricing_service::traits::{
    DefaultDiscountEvaluator, DefaultVoucherEvaluator, DiscountEvaluator, VoucherEvaluator,
};
use kinetix_pricing_service::DbPool;

fn discount(value: Decimal, kind: &str, product: Option<&str>, category: Option<&str>) -> Discount {
    let now = Utc::now();
    return Discount {
        id: Uuid::new_v4(),
        title: "test discount".to_string(),
        discount_type: kind.to_string(),
        value,
        target_product_id: product.map(str::to_string),
        target_category_id: category.map(str::to_string),
        active: true,
        start_time: now - Duration::days(1),
        end_time: now + Duration::days(1),
        created_at: now,
        updated_at: now,
    };
}

fn voucher(
    value: Decimal,
    kind: &str,
    min_spend: Decimal,
    max_discount: Option<Decimal>,
) -> Voucher {
    let now = Utc::now();
    return Voucher {
        id: Uuid::new_v4(),
        code: "PROMO".to_string(),
        title: "test voucher".to_string(),
        discount_type: kind.to_string(),
        value,
        min_spend,
        max_discount,
        quota: 100,
        used_count: 0,
        active: true,
        expires_at: now + Duration::days(7),
        created_at: now,
        updated_at: now,
    };
}

fn item(
    product_id: &str,
    category_id: Option<&str>,
    base_price: Decimal,
    quantity: i32,
) -> PriceItemRequest {
    return PriceItemRequest {
        product_id: product_id.to_string(),
        category_id: category_id.map(str::to_string),
        base_price,
        quantity,
    };
}

#[test]
fn discount_targeted_at_a_product_matches_only_that_product() {
    let evaluator = DefaultDiscountEvaluator;
    let d = discount(dec!(10.00), "PERCENTAGE", Some("SKU-1"), None);

    assert!(evaluator.matches_item(&d, &item("SKU-1", None, dec!(100.00), 1)));
    assert!(!evaluator.matches_item(&d, &item("SKU-2", None, dec!(100.00), 1)));
}

#[test]
fn discount_targeted_at_a_category_matches_only_that_category() {
    let evaluator = DefaultDiscountEvaluator;
    let d = discount(dec!(10.00), "PERCENTAGE", None, Some("HIJAB"));

    assert!(evaluator.matches_item(&d, &item("SKU-1", Some("HIJAB"), dec!(100.00), 1)));
    assert!(!evaluator.matches_item(&d, &item("SKU-1", Some("SHOES"), dec!(100.00), 1)));
    assert!(!evaluator.matches_item(&d, &item("SKU-1", None, dec!(100.00), 1)));
}

#[test]
fn discount_with_no_target_is_global() {
    let evaluator = DefaultDiscountEvaluator;
    let d = discount(dec!(10.00), "PERCENTAGE", None, None);

    assert!(evaluator.matches_item(&d, &item("anything", None, dec!(100.00), 1)));
}

#[test]
fn percentage_savings_are_a_share_of_the_base_price() {
    let evaluator = DefaultDiscountEvaluator;
    let d = discount(dec!(20.00), "PERCENTAGE", None, None);

    assert_eq!(evaluator.calculate_savings(&d, dec!(100.00)), dec!(20.0000));
}

#[test]
fn fixed_savings_never_exceed_the_base_price() {
    let evaluator = DefaultDiscountEvaluator;
    let d = discount(dec!(500.00), "FIXED", None, None);

    assert_eq!(evaluator.calculate_savings(&d, dec!(100.00)), dec!(100.00));
}

#[test]
fn voucher_below_min_spend_is_not_eligible() {
    let evaluator = DefaultVoucherEvaluator;
    let v = voucher(dec!(50.00), "FIXED", dec!(200.00), None);

    assert!(!evaluator.is_eligible(&v, dec!(150.00)));
    assert!(evaluator.is_eligible(&v, dec!(200.00)));
}

#[test]
fn expired_voucher_is_not_eligible() {
    let evaluator = DefaultVoucherEvaluator;
    let mut v = voucher(dec!(50.00), "FIXED", dec!(0.00), None);
    v.expires_at = Utc::now() - Duration::hours(1);

    assert!(!evaluator.is_eligible(&v, dec!(1000.00)));
}

#[test]
fn voucher_with_its_quota_used_up_is_not_eligible() {
    let evaluator = DefaultVoucherEvaluator;
    let mut v = voucher(dec!(50.00), "FIXED", dec!(0.00), None);
    v.used_count = v.quota;

    assert!(!evaluator.is_eligible(&v, dec!(1000.00)));
}

#[test]
fn percentage_voucher_discount_is_capped_by_max_discount() {
    let evaluator = DefaultVoucherEvaluator;
    let v = voucher(dec!(50.00), "PERCENTAGE", dec!(0.00), Some(dec!(25000.00)));

    assert_eq!(
        evaluator.calculate_discount(&v, dec!(100000.00)),
        dec!(25000.00)
    );
}

#[test]
fn voucher_discount_never_exceeds_the_subtotal() {
    let evaluator = DefaultVoucherEvaluator;
    let v = voucher(dec!(500.00), "FIXED", dec!(0.00), None);

    assert_eq!(evaluator.calculate_discount(&v, dec!(100.00)), dec!(100.00));
}

#[test]
fn an_ineligible_voucher_pays_out_nothing() {
    let evaluator = DefaultVoucherEvaluator;
    let v = voucher(dec!(50.00), "FIXED", dec!(200.00), None);

    assert_eq!(evaluator.calculate_discount(&v, dec!(150.00)), dec!(0.00));
}

struct FakeDiscounts(Vec<Discount>);
struct FakeVouchers(Option<Voucher>);
struct FakeFlashSales(Option<FlashSale>);

#[async_trait]
impl DiscountRepositoryPort for FakeDiscounts {
    async fn find_all_active(&self, _pool: &DbPool) -> Result<Vec<Discount>, AppError> {
        return Ok(self.0.clone());
    }
    async fn create(
        &self,
        _pool: &DbPool,
        _req: CreateDiscountRequest,
    ) -> Result<Discount, AppError> {
        unimplemented!("not used by these tests");
    }
}

#[async_trait]
impl VoucherRepositoryPort for FakeVouchers {
    async fn find_by_code(&self, _pool: &DbPool, _code: &str) -> Result<Option<Voucher>, AppError> {
        return Ok(self.0.clone());
    }
    async fn create(
        &self,
        _pool: &DbPool,
        _req: CreateVoucherRequest,
    ) -> Result<Voucher, AppError> {
        unimplemented!("not used by these tests");
    }
}

#[async_trait]
impl FlashSaleRepositoryPort for FakeFlashSales {
    async fn find_active_for_product(
        &self,
        _pool: &DbPool,
        _product_id: &str,
    ) -> Result<Option<FlashSale>, AppError> {
        return Ok(self.0.clone());
    }
    async fn create(
        &self,
        _pool: &DbPool,
        _req: CreateFlashSaleRequest,
    ) -> Result<FlashSale, AppError> {
        unimplemented!("not used by these tests");
    }
}

fn unconnected_pool() -> DbPool {
    return create_pool("postgres://unused:unused@127.0.0.1:1/unused");
}

struct FakeShippingRates(Option<ShippingRate>);

#[async_trait]
impl ShippingRateRepositoryPort for FakeShippingRates {
    async fn find_by_tier(
        &self,
        _pool: &DbPool,
        _tier: &str,
    ) -> Result<Option<ShippingRate>, AppError> {
        return Ok(self.0.clone());
    }
}

fn rate(
    tier: &str,
    base: rust_decimal::Decimal,
    per_km: rust_decimal::Decimal,
    per_kg: rust_decimal::Decimal,
    per_kg_per_100km: rust_decimal::Decimal,
) -> ShippingRate {
    let now = Utc::now();

    return ShippingRate {
        service_tier: tier.to_string(),
        base_fee: base,
        per_km_fee: per_km,
        per_kg_fee: per_kg,
        per_kg_per_100km_fee: per_kg_per_100km,
        currency: "IDR".to_string(),
        active: true,
        created_at: now,
        updated_at: now,
    };
}

fn service(
    discounts: Vec<Discount>,
    v: Option<Voucher>,
    flash: Option<FlashSale>,
) -> PricingService<FakeDiscounts, FakeVouchers, FakeFlashSales, FakeShippingRates> {
    return service_with_rate(discounts, v, flash, None);
}

fn service_with_rate(
    discounts: Vec<Discount>,
    v: Option<Voucher>,
    flash: Option<FlashSale>,
    shipping: Option<ShippingRate>,
) -> PricingService<FakeDiscounts, FakeVouchers, FakeFlashSales, FakeShippingRates> {
    return PricingService::new(
        FakeDiscounts(discounts),
        FakeVouchers(v),
        FakeFlashSales(flash),
        FakeShippingRates(shipping),
        Box::new(DefaultDiscountEvaluator),
        Box::new(DefaultVoucherEvaluator),
    );
}

#[tokio::test]
async fn a_flash_sale_wins_over_a_discount_on_the_same_item() {
    let now = Utc::now();
    let flash_id = Uuid::new_v4();
    let flash = FlashSale {
        id: flash_id,
        title: "flash".to_string(),
        product_id: "SKU-1".to_string(),
        flash_price: dec!(49.99),
        stock_limit: 10,
        stock_sold: 0,
        active: true,
        start_time: now - Duration::hours(1),
        end_time: now + Duration::hours(1),
        created_at: now,
        updated_at: now,
    };
    let svc = service(
        vec![discount(dec!(10.00), "PERCENTAGE", None, None)],
        None,
        Some(flash),
    );

    let res = svc
        .calculate_price(
            &unconnected_pool(),
            CalculatePriceRequest {
                items: vec![item("SKU-1", None, dec!(100.00), 1)],
                voucher_code: None,
                base_shipping_fee: None,
                payment_method: None,
                shipping: None,
            },
        )
        .await
        .expect("calculate_price should succeed");

    assert_eq!(res.items[0].final_unit_price, dec!(49.99));
    assert_eq!(
        res.items[0].applied_flash_sale.as_deref(),
        Some(flash_id.to_string().as_str())
    );
    assert!(res.items[0].applied_discount.is_none());
}

#[tokio::test]
async fn the_wallet_payment_discount_is_capped_at_25000() {
    let svc = service(vec![], None, None);

    let res = svc
        .calculate_price(
            &unconnected_pool(),
            CalculatePriceRequest {
                items: vec![item("SKU-1", None, dec!(1_000_000.00), 1)],
                voucher_code: None,
                base_shipping_fee: None,
                payment_method: Some("INTERNAL_WALLET".to_string()),
                shipping: None,
            },
        )
        .await
        .expect("calculate_price should succeed");

    assert_eq!(res.payment_discount, dec!(25000.00));
    assert_eq!(res.final_total, dec!(975000.00));
}

#[tokio::test]
async fn shipping_is_never_charged_below_zero() {
    let mut v = voucher(dec!(100.00), "FIXED", dec!(0.00), None);
    v.code = "FREE_SHIP".to_string();
    let svc = service(vec![], Some(v), None);

    let res = svc
        .calculate_price(
            &unconnected_pool(),
            CalculatePriceRequest {
                items: vec![item("SKU-1", None, dec!(50_000.00), 1)],
                voucher_code: Some("FREE_SHIP".to_string()),
                base_shipping_fee: Some(dec!(20.00)),
                payment_method: None,
                shipping: None,
            },
        )
        .await
        .expect("calculate_price should succeed");

    assert_eq!(res.final_shipping_fee, dec!(0.00));
    assert_eq!(res.shipping_discount, dec!(20.00));
}

fn quote(tier: &str, distance_km: rust_decimal::Decimal, grams: i64) -> ShippingQuoteRequest {
    return ShippingQuoteRequest {
        service_tier: tier.to_string(),
        distance_km,
        total_weight_grams: grams,
    };
}

async fn fee_for(rate_row: ShippingRate, quoted: ShippingQuoteRequest) -> rust_decimal::Decimal {
    let svc = service_with_rate(vec![], None, None, Some(rate_row));

    let res = svc
        .calculate_price(
            &unconnected_pool(),
            CalculatePriceRequest {
                items: vec![item("SKU-1", None, dec!(10_000.00), 1)],
                voucher_code: None,
                base_shipping_fee: None,
                payment_method: None,
                shipping: Some(quoted),
            },
        )
        .await
        .expect("calculate_price should succeed");

    return res.base_shipping_fee;
}

#[tokio::test]
async fn instant_costs_what_matching_used_to_charge() {
    let fee = fee_for(
        rate("KINETIX_INSTANT", dec!(15000), dec!(3000), dec!(0), dec!(0)),
        quote("KINETIX_INSTANT", dec!(8.5), 2_000),
    )
    .await;

    assert_eq!(fee, dec!(40500.00));
}

#[tokio::test]
async fn sameday_costs_what_matching_used_to_charge() {
    let fee = fee_for(
        rate("KINETIX_SAMEDAY", dec!(12000), dec!(2000), dec!(0), dec!(0)),
        quote("KINETIX_SAMEDAY", dec!(20), 5_000),
    )
    .await;

    assert_eq!(fee, dec!(52000.00));
}

#[tokio::test]
async fn cargo_is_priced_by_weight_and_not_by_distance() {
    let fee = fee_for(
        rate("KINETIX_CARGO", dec!(25000), dec!(0), dec!(1000), dec!(0)),
        quote("KINETIX_CARGO", dec!(300), 12_000),
    )
    .await;

    assert_eq!(fee, dec!(37000.00));
}

#[tokio::test]
async fn regular_charges_by_weight_across_distance_bands() {
    let fee = fee_for(
        rate("KINETIX_REGULAR", dec!(9000), dec!(0), dec!(0), dec!(1500)),
        quote("KINETIX_REGULAR", dec!(250), 4_000),
    )
    .await;

    assert_eq!(fee, dec!(24000.00));
}

#[tokio::test]
async fn a_short_regular_journey_still_pays_one_band() {
    let fee = fee_for(
        rate("KINETIX_REGULAR", dec!(9000), dec!(0), dec!(0), dec!(1500)),
        quote("KINETIX_REGULAR", dec!(30), 4_000),
    )
    .await;

    assert_eq!(fee, dec!(15000.00));
}

#[tokio::test]
async fn a_tier_with_no_rate_is_refused_rather_than_shipped_free() {
    let svc = service_with_rate(vec![], None, None, None);

    let err = svc
        .calculate_price(
            &unconnected_pool(),
            CalculatePriceRequest {
                items: vec![item("SKU-1", None, dec!(10_000.00), 1)],
                voucher_code: None,
                base_shipping_fee: None,
                payment_method: None,
                shipping: Some(quote("KINETIX_TELEPORT", dec!(5), 1_000)),
            },
        )
        .await
        .expect_err("a tier with no rate must not be priced");

    assert!(
        err.to_string().contains("KINETIX_TELEPORT"),
        "the refusal should name the tier it has no rate for, got: {err}"
    );
}

#[tokio::test]
async fn a_shipping_voucher_discounts_the_fee_this_service_computed() {
    let mut v = voucher(dec!(100.00), "FIXED", dec!(0.00), None);
    v.code = "FREE_SHIP".to_string();

    let svc = service_with_rate(
        vec![],
        Some(v),
        None,
        Some(rate(
            "KINETIX_INSTANT",
            dec!(15000),
            dec!(3000),
            dec!(0),
            dec!(0),
        )),
    );

    let res = svc
        .calculate_price(
            &unconnected_pool(),
            CalculatePriceRequest {
                items: vec![item("SKU-1", None, dec!(50_000.00), 1)],
                voucher_code: Some("FREE_SHIP".to_string()),
                base_shipping_fee: None,
                payment_method: None,
                shipping: Some(quote("KINETIX_INSTANT", dec!(1), 1_000)),
            },
        )
        .await
        .expect("calculate_price should succeed");

    assert_eq!(res.base_shipping_fee, dec!(18000.00));
    assert_eq!(res.shipping_discount, dec!(100.00));
    assert_eq!(res.final_shipping_fee, dec!(17900.00));
}

#[tokio::test]
async fn quoting_several_tiers_prices_the_ones_it_has_rates_for() {
    let svc = service_with_rate(
        vec![],
        None,
        None,
        Some(rate(
            "KINETIX_INSTANT",
            dec!(15000),
            dec!(3000),
            dec!(0),
            dec!(0),
        )),
    );

    let quotes = svc
        .quote_shipping(
            &unconnected_pool(),
            vec![
                quote("KINETIX_INSTANT", dec!(2), 1_000),
                quote("KINETIX_INSTANT", dec!(4), 1_000),
            ],
        )
        .await
        .expect("quote_shipping should succeed");

    assert_eq!(quotes.len(), 2);
    assert_eq!(quotes[0].base_shipping_fee, dec!(21000.00));
    assert_eq!(quotes[1].base_shipping_fee, dec!(27000.00));
    assert!(quotes.iter().all(|q| q.priced));
}

#[tokio::test]
async fn a_tier_with_no_rate_comes_back_unpriced_rather_than_free() {
    let svc = service_with_rate(vec![], None, None, None);

    let quotes = svc
        .quote_shipping(
            &unconnected_pool(),
            vec![quote("KINETIX_TELEPORT", dec!(2), 1_000)],
        )
        .await
        .expect("quote_shipping should succeed");

    assert_eq!(quotes.len(), 1);
    assert!(!quotes[0].priced);
    assert_eq!(quotes[0].base_shipping_fee, dec!(0.00));
}
