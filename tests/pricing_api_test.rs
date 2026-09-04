use async_trait::async_trait;
use chrono::{Duration, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use uuid::Uuid;

use kinetix_pricing_service::db::create_pool;
use kinetix_pricing_service::error::AppError;
use kinetix_pricing_service::models::{
    CalculatePriceRequest, CreateDiscountRequest, CreateFlashSaleRequest, CreateVoucherRequest,
    Discount, FlashSale, PriceItemRequest, Voucher,
};
use kinetix_pricing_service::repositories::{
    DiscountRepositoryPort, FlashSaleRepositoryPort, VoucherRepositoryPort,
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

fn voucher(value: Decimal, kind: &str, min_spend: Decimal, max_discount: Option<Decimal>) -> Voucher {
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

fn item(product_id: &str, category_id: Option<&str>, base_price: Decimal, quantity: i32) -> PriceItemRequest {
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

    // 50% of 100000 is 50000, but the voucher caps its own payout at 25000.
    assert_eq!(evaluator.calculate_discount(&v, dec!(100000.00)), dec!(25000.00));
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

// ── the service, with faked repositories ────────────────────────────────────────────────────

struct FakeDiscounts(Vec<Discount>);
struct FakeVouchers(Option<Voucher>);
struct FakeFlashSales(Option<FlashSale>);

#[async_trait]
impl DiscountRepositoryPort for FakeDiscounts {
    async fn find_all_active(&self, _pool: &DbPool) -> Result<Vec<Discount>, AppError> {
        return Ok(self.0.clone());
    }
    async fn create(&self, _pool: &DbPool, _req: CreateDiscountRequest) -> Result<Discount, AppError> {
        unimplemented!("not used by these tests");
    }
}

#[async_trait]
impl VoucherRepositoryPort for FakeVouchers {
    async fn find_by_code(&self, _pool: &DbPool, _code: &str) -> Result<Option<Voucher>, AppError> {
        return Ok(self.0.clone());
    }
    async fn create(&self, _pool: &DbPool, _req: CreateVoucherRequest) -> Result<Voucher, AppError> {
        unimplemented!("not used by these tests");
    }
}

#[async_trait]
impl FlashSaleRepositoryPort for FakeFlashSales {
    async fn find_active_for_product(&self, _pool: &DbPool, _product_id: &str) -> Result<Option<FlashSale>, AppError> {
        return Ok(self.0.clone());
    }
    async fn create(&self, _pool: &DbPool, _req: CreateFlashSaleRequest) -> Result<FlashSale, AppError> {
        unimplemented!("not used by these tests");
    }
}

// deadpool connects lazily, so this pool is never opened. The fakes above ignore it entirely.
fn unconnected_pool() -> DbPool {
    return create_pool("postgres://unused:unused@127.0.0.1:1/unused");
}

fn service(
    discounts: Vec<Discount>,
    v: Option<Voucher>,
    flash: Option<FlashSale>,
) -> PricingService<FakeDiscounts, FakeVouchers, FakeFlashSales> {
    return PricingService::new(
        FakeDiscounts(discounts),
        FakeVouchers(v),
        FakeFlashSales(flash),
        Box::new(DefaultDiscountEvaluator),
        Box::new(DefaultVoucherEvaluator),
    );
}

#[tokio::test]
async fn a_flash_sale_wins_over_a_discount_on_the_same_item() {
    let now = Utc::now();
    let flash = FlashSale {
        id: Uuid::new_v4(),
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
    let svc = service(vec![discount(dec!(10.00), "PERCENTAGE", None, None)], None, Some(flash));

    let res = svc
        .calculate_price(
            &unconnected_pool(),
            CalculatePriceRequest {
                items: vec![item("SKU-1", None, dec!(100.00), 1)],
                voucher_code: None,
                base_shipping_fee: None,
                payment_method: None,
            },
        )
        .await
        .expect("calculate_price should succeed");

    assert_eq!(res.items[0].final_unit_price, dec!(49.99));
    assert_eq!(res.items[0].applied_flash_sale.as_deref(), Some("flash"));
    // The 10% discount must not also apply — a flash sale suppresses it.
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
            },
        )
        .await
        .expect("calculate_price should succeed");

    // 5% of 1,000,000 is 50,000, which the cap holds down to 25,000.
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
            },
        )
        .await
        .expect("calculate_price should succeed");

    // The voucher is worth more than the shipping fee; the fee floors at zero rather than
    // turning into a credit.
    assert_eq!(res.final_shipping_fee, dec!(0.00));
    assert_eq!(res.shipping_discount, dec!(20.00));
}
