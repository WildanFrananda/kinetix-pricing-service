use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use kinetix_pricing_service::clients::{
    MerchantDirectoryPort, MerchantStanding, PeerError, ProductDirectoryPort,
};
use kinetix_pricing_service::db::create_pool;
use kinetix_pricing_service::error::AppError;
use kinetix_pricing_service::guards::{AdminOrMerchantGuard, IdempotencyKey, JsonBody};
use kinetix_pricing_service::models::{
    CreateDiscountRequest, CreateFlashSaleRequest, CreateVoucherRequest, DiscountType,
};
use kinetix_pricing_service::routes::discount_routes::create_discount;
use kinetix_pricing_service::routes::flash_sale_routes::create_flash_sale;
use kinetix_pricing_service::routes::voucher_routes::create_voucher;
use kinetix_pricing_service::services::promotion_terms::EARLIEST_STORABLE_UNIX_SECONDS;
use kinetix_pricing_service::services::{check_discount, check_voucher, PromotionService};
use kinetix_pricing_service::DbPool;

type Spoil<T> = Box<dyn Fn(&mut T)>;
type Case<T> = (&'static str, &'static str, Spoil<T>);

struct UnreachableIdentity;
struct UnreachableCatalog;

#[async_trait]
impl MerchantDirectoryPort for UnreachableIdentity {
    async fn standing_of(
        &self,
        _principal_id: &str,
    ) -> Result<Option<MerchantStanding>, PeerError> {
        return Err(PeerError::Unavailable(
            "identity did not answer".to_string(),
        ));
    }
}

#[async_trait]
impl ProductDirectoryPort for UnreachableCatalog {
    async fn owner_of(&self, _sku: &str) -> Result<Option<String>, PeerError> {
        return Err(PeerError::Unavailable("catalog did not answer".to_string()));
    }
}

fn promotions() -> PromotionService {
    return PromotionService::new(Arc::new(UnreachableIdentity), Arc::new(UnreachableCatalog));
}

fn unconnected_pool() -> DbPool {
    return create_pool("postgres://unused:unused@127.0.0.1:1/unused");
}

fn admin() -> AdminOrMerchantGuard {
    return AdminOrMerchantGuard {
        principal_id: "admin-principal".to_string(),
        role: "admin".to_string(),
    };
}

fn key() -> IdempotencyKey {
    return IdempotencyKey("terms-test".to_string());
}

fn voucher() -> CreateVoucherRequest {
    return CreateVoucherRequest {
        code: "SAVE10".to_string(),
        title: "Ten off".to_string(),
        discount_type: DiscountType::Percentage,
        value: dec!(10),
        min_spend: dec!(50000),
        max_discount: Some(dec!(25000)),
        quota: 100,
        expires_at: Utc::now() + Duration::days(7),
    };
}

fn discount() -> CreateDiscountRequest {
    return CreateDiscountRequest {
        title: "Shoes week".to_string(),
        discount_type: DiscountType::Fixed,
        value: dec!(5000),
        target_product_id: None,
        target_category_id: Some("shoes".to_string()),
        start_time: Utc::now() - Duration::hours(1),
        end_time: Utc::now() + Duration::days(7),
    };
}

fn flash_sale() -> CreateFlashSaleRequest {
    return CreateFlashSaleRequest {
        title: "Midnight".to_string(),
        product_id: "SKU-1".to_string(),
        flash_price: dec!(9900),
        stock_limit: 10,
        start_time: Utc::now() + Duration::hours(1),
        end_time: Utc::now() + Duration::hours(2),
    };
}

async fn voucher_answer(req: CreateVoucherRequest) -> AppError {
    let pool = unconnected_pool();
    let promotions = promotions();

    return create_voucher(
        admin(),
        key(),
        (&pool).into(),
        (&promotions).into(),
        JsonBody(req),
    )
    .await
    .expect_err("an unconnected pool cannot create anything");
}

async fn discount_answer(req: CreateDiscountRequest) -> AppError {
    let pool = unconnected_pool();
    let promotions = promotions();

    return create_discount(
        admin(),
        key(),
        (&pool).into(),
        (&promotions).into(),
        JsonBody(req),
    )
    .await
    .expect_err("an unconnected pool cannot create anything");
}

async fn flash_sale_answer(req: CreateFlashSaleRequest) -> AppError {
    let pool = unconnected_pool();
    let promotions = promotions();

    return create_flash_sale(
        admin(),
        key(),
        (&pool).into(),
        (&promotions).into(),
        JsonBody(req),
    )
    .await
    .expect_err("an unconnected pool cannot create anything");
}

fn refusal_names(answer: AppError, field: &str, case: &str) {
    match answer {
        AppError::BadRequest(message) => {
            assert!(
                message.contains(field),
                "{case}: the refusal does not name {field}: {message}"
            );
        }
        other => panic!("{case}: expected a 400 naming {field}, got {other:?}"),
    }
}

fn chars(count: usize) -> String {
    return "é".repeat(count);
}

#[tokio::test]
async fn a_valid_voucher_passes_every_check_and_only_then_reaches_the_database() {
    let answer = voucher_answer(voucher()).await;

    assert!(matches!(answer, AppError::Pool(_)), "{answer:?}");
}

#[tokio::test]
async fn every_bad_voucher_field_is_refused_before_the_database_is_touched() {
    let cases: Vec<Case<CreateVoucherRequest>> = vec![
        (
            "blank code",
            "code",
            Box::new(|v| v.code = "   ".to_string()),
        ),
        (
            "code with spaces around it",
            "code",
            Box::new(|v| v.code = " SAVE10".to_string()),
        ),
        (
            "code longer than its column",
            "code",
            Box::new(|v| v.code = chars(101)),
        ),
        (
            "code with a NUL",
            "code",
            Box::new(|v| v.code = "SAVE\0".to_string()),
        ),
        (
            "blank title",
            "title",
            Box::new(|v| v.title = String::new()),
        ),
        (
            "title longer than its column",
            "title",
            Box::new(|v| v.title = chars(256)),
        ),
        (
            "title with a NUL",
            "title",
            Box::new(|v| v.title = "a\0b".to_string()),
        ),
        ("zero value", "value", Box::new(|v| v.value = dec!(0))),
        ("negative value", "value", Box::new(|v| v.value = dec!(-1))),
        (
            "value below a cent",
            "value",
            Box::new(|v| v.value = dec!(1.005)),
        ),
        (
            "percentage above 100",
            "percentage",
            Box::new(|v| v.value = dec!(100.01)),
        ),
        (
            "value too large for its column",
            "value",
            Box::new(|v| {
                v.discount_type = DiscountType::Fixed;
                v.value = dec!(10_000_000_000);
            }),
        ),
        (
            "negative min_spend",
            "min_spend",
            Box::new(|v| v.min_spend = dec!(-0.01)),
        ),
        (
            "min_spend below a cent",
            "min_spend",
            Box::new(|v| v.min_spend = dec!(0.001)),
        ),
        (
            "min_spend too large",
            "min_spend",
            Box::new(|v| v.min_spend = dec!(99_999_999_999)),
        ),
        (
            "zero max_discount",
            "max_discount",
            Box::new(|v| v.max_discount = Some(dec!(0))),
        ),
        (
            "negative max_discount",
            "max_discount",
            Box::new(|v| {
                v.max_discount = Some(dec!(-50000));
            }),
        ),
        (
            "max_discount below a cent",
            "max_discount",
            Box::new(|v| {
                v.max_discount = Some(dec!(0.005));
            }),
        ),
        ("zero quota", "quota", Box::new(|v| v.quota = 0)),
        ("negative quota", "quota", Box::new(|v| v.quota = -3)),
        (
            "expiry in the past",
            "expires_at",
            Box::new(|v| {
                v.expires_at = Utc::now() - Duration::seconds(1);
            }),
        ),
    ];

    for (case, field, spoil) in cases {
        let mut req = voucher();
        spoil(&mut req);
        refusal_names(voucher_answer(req).await, field, case);
    }
}

#[tokio::test]
async fn a_valid_discount_passes_every_check_and_only_then_reaches_the_database() {
    let answer = discount_answer(discount()).await;

    assert!(matches!(answer, AppError::Pool(_)), "{answer:?}");
}

#[tokio::test]
async fn every_bad_discount_field_is_refused_before_anyone_is_asked() {
    let ancient = DateTime::from_timestamp(EARLIEST_STORABLE_UNIX_SECONDS - 1, 0)
        .expect("a representable time");
    let cases: Vec<Case<CreateDiscountRequest>> = vec![
        (
            "blank title",
            "title",
            Box::new(|d| d.title = " ".to_string()),
        ),
        (
            "title longer than its column",
            "title",
            Box::new(|d| d.title = chars(256)),
        ),
        ("zero value", "value", Box::new(|d| d.value = dec!(0))),
        (
            "value below a cent",
            "value",
            Box::new(|d| d.value = dec!(0.999)),
        ),
        (
            "value too large",
            "value",
            Box::new(|d| d.value = dec!(12_345_678_901)),
        ),
        (
            "percentage above 100",
            "percentage",
            Box::new(|d| {
                d.discount_type = DiscountType::Percentage;
                d.value = dec!(150);
            }),
        ),
        (
            "blank product",
            "target_product_id",
            Box::new(|d| {
                d.target_product_id = Some(String::new());
            }),
        ),
        (
            "product longer than its column",
            "target_product_id",
            Box::new(|d| {
                d.target_product_id = Some(chars(101));
            }),
        ),
        (
            "blank category",
            "target_category_id",
            Box::new(|d| {
                d.target_category_id = Some(" ".to_string());
            }),
        ),
        (
            "window that ends as it starts",
            "start_time",
            Box::new(|d| d.end_time = d.start_time),
        ),
        (
            "window already over",
            "end_time",
            Box::new(|d| {
                d.start_time = Utc::now() - Duration::days(2);
                d.end_time = Utc::now() - Duration::days(1);
            }),
        ),
        (
            "start before any storable time",
            "start_time",
            Box::new(move |d| {
                d.start_time = ancient;
            }),
        ),
    ];

    for (case, field, spoil) in cases {
        let mut req = discount();
        spoil(&mut req);
        refusal_names(discount_answer(req).await, field, case);
    }
}

#[tokio::test]
async fn a_valid_flash_sale_passes_every_check_and_then_waits_for_catalog() {
    let answer = flash_sale_answer(flash_sale()).await;

    assert!(matches!(answer, AppError::Unavailable(_)), "{answer:?}");
}

#[tokio::test]
async fn every_bad_flash_sale_field_is_refused_before_anyone_is_asked() {
    let cases: Vec<Case<CreateFlashSaleRequest>> = vec![
        (
            "blank title",
            "title",
            Box::new(|f| f.title = String::new()),
        ),
        (
            "title longer than its column",
            "title",
            Box::new(|f| f.title = chars(256)),
        ),
        (
            "blank product",
            "product_id",
            Box::new(|f| f.product_id = String::new()),
        ),
        (
            "product longer than its column",
            "product_id",
            Box::new(|f| {
                f.product_id = chars(101);
            }),
        ),
        (
            "zero price",
            "flash_price",
            Box::new(|f| f.flash_price = dec!(0)),
        ),
        (
            "negative price",
            "flash_price",
            Box::new(|f| f.flash_price = dec!(-1)),
        ),
        (
            "price below a cent",
            "flash_price",
            Box::new(|f| f.flash_price = dec!(9.999)),
        ),
        (
            "price too large",
            "flash_price",
            Box::new(|f| {
                f.flash_price = dec!(10_000_000_000.00);
            }),
        ),
        ("zero stock", "stock_limit", Box::new(|f| f.stock_limit = 0)),
        (
            "ends before it starts",
            "start_time",
            Box::new(|f| {
                f.end_time = f.start_time - Duration::minutes(1);
            }),
        ),
        (
            "already over",
            "end_time",
            Box::new(|f| {
                f.start_time = Utc::now() - Duration::hours(3);
                f.end_time = Utc::now() - Duration::hours(2);
            }),
        ),
    ];

    for (case, field, spoil) in cases {
        let mut req = flash_sale();
        spoil(&mut req);
        refusal_names(flash_sale_answer(req).await, field, case);
    }
}

#[test]
fn the_largest_amount_each_column_holds_is_accepted_and_trailing_zeros_are_not_extra_places() {
    let mut req = voucher();
    req.discount_type = DiscountType::Fixed;
    req.value = dec!(9_999_999_999.99);
    req.min_spend = dec!(9_999_999_999.990);
    req.max_discount = Some(dec!(0.01));

    assert!(check_voucher(&req, Utc::now()).is_ok());
}

#[test]
fn titles_and_ids_are_measured_in_characters_as_the_columns_are() {
    let mut req = discount();
    req.title = chars(255);
    req.target_product_id = Some(chars(100));

    assert!(check_discount(&req, Utc::now()).is_ok());
}

#[test]
fn the_earliest_time_postgres_stores_is_accepted_and_one_second_earlier_is_not() {
    let earliest =
        DateTime::from_timestamp(EARLIEST_STORABLE_UNIX_SECONDS, 0).expect("a representable time");
    let mut req = discount();

    req.start_time = earliest;
    assert!(check_discount(&req, Utc::now()).is_ok());

    req.start_time = earliest - Duration::seconds(1);
    assert!(check_discount(&req, Utc::now()).is_err());
}

#[test]
fn a_percentage_of_exactly_one_hundred_is_a_valid_voucher() {
    let mut req = voucher();
    req.value = Decimal::ONE_HUNDRED;

    assert!(check_voucher(&req, Utc::now()).is_ok());
}
