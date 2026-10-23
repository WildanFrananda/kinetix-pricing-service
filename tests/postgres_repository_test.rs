use std::sync::{Arc, Once};

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use diesel::prelude::*;
use diesel::{Connection, PgConnection};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use rust_decimal_macros::dec;
use uuid::Uuid;

use kinetix_pricing_service::clients::{
    MerchantDirectoryPort, MerchantStanding, PeerError, ProductDirectoryPort,
};
use kinetix_pricing_service::db::create_pool;
use kinetix_pricing_service::error::AppError;
use kinetix_pricing_service::grpc::PricingGrpcServer;
use kinetix_pricing_service::guards::{
    AdminActionTarget, AdminOrMerchantGuard, IdempotencyKey, JsonBody, OptionalCaller,
};
use kinetix_pricing_service::models::{
    CreateDiscountRequest, CreateFlashSaleRequest, CreateVoucherRequest, DiscountType,
    IdempotentCreation, Voucher, VoucherView,
};
use kinetix_pricing_service::proto::pricing::v1::pricing_service_server::PricingService as _;
use kinetix_pricing_service::proto::pricing::v1::RedeemVoucherRequest;
use kinetix_pricing_service::repositories::{
    DiscountRepository, DiscountRepositoryPort, FlashSaleRepository, FlashSaleRepositoryPort,
    QuotaOutcome, QuotaRepository, VoucherRepository, VoucherRepositoryPort,
};
use kinetix_pricing_service::routes::voucher_routes::{apply_voucher, create_voucher, get_voucher};
use kinetix_pricing_service::services::promotion_terms::EARLIEST_STORABLE_UNIX_SECONDS;
use kinetix_pricing_service::services::PromotionService;
use kinetix_pricing_service::DbPool;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");
const DATABASE_VARIABLE: &str = "KINETIX_TEST_POSTGRES";

static MIGRATED: Once = Once::new();

fn database_url() -> Option<String> {
    if let Ok(url) = std::env::var(DATABASE_VARIABLE) {
        MIGRATED.call_once(|| {
            let mut connection =
                PgConnection::establish(&url).expect("the test database is not reachable");
            connection
                .run_pending_migrations(MIGRATIONS)
                .expect("the migrations did not apply");
        });
        return Some(url);
    }

    assert!(
        std::env::var("CI").is_err(),
        "CI must set {DATABASE_VARIABLE}; without it these tests prove nothing"
    );
    return None;
}

fn sql(url: &str) -> PgConnection {
    return PgConnection::establish(url).expect("the test database is not reachable");
}

struct NoMerchants;
struct NoProducts;

#[async_trait]
impl MerchantDirectoryPort for NoMerchants {
    async fn standing_of(
        &self,
        _principal_id: &str,
    ) -> Result<Option<MerchantStanding>, PeerError> {
        return Ok(None);
    }
}

#[async_trait]
impl ProductDirectoryPort for NoProducts {
    async fn owner_of(&self, _sku: &str) -> Result<Option<String>, PeerError> {
        return Ok(None);
    }
}

fn promotions() -> PromotionService {
    return PromotionService::new(Arc::new(NoMerchants), Arc::new(NoProducts));
}

fn admin(principal_id: &str) -> AdminOrMerchantGuard {
    return AdminOrMerchantGuard {
        principal_id: principal_id.to_string(),
        role: "admin".to_string(),
    };
}

fn unique(prefix: &str) -> String {
    return format!("{prefix}-{}", Uuid::new_v4()).to_uppercase();
}

fn voucher(code: &str) -> CreateVoucherRequest {
    return CreateVoucherRequest {
        code: code.to_string(),
        title: "Ten off".to_string(),
        discount_type: DiscountType::Fixed,
        value: dec!(10000),
        min_spend: dec!(0),
        max_discount: None,
        quota: 1,
        expires_at: Utc::now() + Duration::days(1),
    };
}

fn creation(principal_id: &str, key: &str) -> IdempotentCreation {
    return IdempotentCreation {
        principal_id: principal_id.to_string(),
        idempotency_key: key.to_string(),
    };
}

async fn create(
    pool: &DbPool,
    principal_id: &str,
    key: &str,
    req: CreateVoucherRequest,
) -> Result<(Voucher, bool), AppError> {
    let promotions = promotions();
    let created = create_voucher(
        admin(principal_id),
        AdminActionTarget::default(),
        IdempotencyKey(key.to_string()),
        pool.into(),
        (&promotions).into(),
        JsonBody(req),
    )
    .await?;

    return Ok((created.body.into_inner(), created.replayed));
}

fn vouchers_with_code(url: &str, wanted: &str) -> i64 {
    use kinetix_pricing_service::schema::vouchers::dsl::*;

    return vouchers
        .filter(code.eq(wanted))
        .count()
        .get_result(&mut sql(url))
        .expect("count vouchers");
}

#[tokio::test]
async fn a_duplicate_voucher_code_is_409_whoever_asks_and_whoever_owns_it() {
    let Some(url) = database_url() else {
        return;
    };
    let pool = create_pool(&url);
    let code = unique("DUP");

    create(&pool, "admin-a", &unique("key"), voucher(&code))
        .await
        .expect("the first voucher is created");

    let same_admin = create(&pool, "admin-a", &unique("key"), voucher(&code)).await;
    assert!(
        matches!(same_admin, Err(AppError::Conflict(_))),
        "{same_admin:?}"
    );

    let another_admin = create(&pool, "admin-b", &unique("key"), voucher(&code)).await;
    assert!(
        matches!(another_admin, Err(AppError::Conflict(_))),
        "{another_admin:?}"
    );

    let a_merchant = VoucherRepository
        .create(
            &pool,
            voucher(&code),
            Some("merchant-b".to_string()),
            creation("seller-b", &unique("key")),
        )
        .await;
    assert!(
        matches!(a_merchant, Err(AppError::Conflict(_))),
        "{a_merchant:?}"
    );

    assert_eq!(vouchers_with_code(&url, &code), 1);
}

#[tokio::test]
async fn a_refused_duplicate_leaves_its_key_free_for_the_corrected_request() {
    let Some(url) = database_url() else {
        return;
    };
    let pool = create_pool(&url);
    let taken = unique("TAKEN");
    let key = unique("key");

    create(&pool, "admin-a", &unique("key"), voucher(&taken))
        .await
        .expect("the first voucher is created");

    let refused = create(&pool, "admin-a", &key, voucher(&taken)).await;
    assert!(matches!(refused, Err(AppError::Conflict(_))), "{refused:?}");

    let corrected = unique("FREE");
    let (made, replayed) = create(&pool, "admin-a", &key, voucher(&corrected))
        .await
        .expect("the key was never spent on the refused attempt");
    assert_eq!(made.code, corrected);
    assert!(!replayed);
}

#[tokio::test]
async fn a_retry_with_the_same_key_and_body_returns_the_first_voucher_and_says_it_is_a_replay() {
    let Some(url) = database_url() else {
        return;
    };
    let pool = create_pool(&url);
    let code = unique("RETRY");
    let key = unique("key");
    let req = voucher(&code);

    let (first, first_replayed) = create(&pool, "admin-a", &key, req.clone())
        .await
        .expect("created");
    let (second, second_replayed) = create(&pool, "admin-a", &key, req).await.expect("replayed");

    assert!(!first_replayed);
    assert!(second_replayed);
    assert_eq!(first.id, second.id);
    assert_eq!(vouchers_with_code(&url, &code), 1);
}

#[tokio::test]
async fn the_same_key_with_a_different_body_is_422_and_creates_nothing() {
    let Some(url) = database_url() else {
        return;
    };
    let pool = create_pool(&url);
    let key = unique("key");
    let other_code = unique("OTHER");

    create(&pool, "admin-a", &key, voucher(&unique("FIRST")))
        .await
        .expect("created");

    let reused = create(&pool, "admin-a", &key, voucher(&other_code)).await;

    assert!(
        matches!(reused, Err(AppError::Unprocessable(_))),
        "{reused:?}"
    );
    assert_eq!(vouchers_with_code(&url, &other_code), 0);
}

#[tokio::test]
async fn a_key_belongs_to_the_principal_that_sent_it() {
    let Some(url) = database_url() else {
        return;
    };
    let pool = create_pool(&url);
    let key = unique("key");

    let (a, _) = create(&pool, "admin-a", &key, voucher(&unique("A")))
        .await
        .expect("created for a");
    let (b, b_replayed) = create(&pool, "admin-b", &key, voucher(&unique("B")))
        .await
        .expect("created for b");

    assert_ne!(a.id, b.id);
    assert!(!b_replayed);
}

#[tokio::test]
async fn two_racing_creates_with_one_key_make_one_voucher_and_both_get_it() {
    let Some(url) = database_url() else {
        return;
    };
    let pool = create_pool(&url);
    let code = unique("RACE");
    let key = unique("key");

    let req = voucher(&code);

    let (left, right) = tokio::join!(
        create(&pool, "admin-a", &key, req.clone()),
        create(&pool, "admin-a", &key, req),
    );
    let (left, left_replayed) = left.expect("left answered");
    let (right, right_replayed) = right.expect("right answered");

    assert_eq!(left.id, right.id);
    assert!(left_replayed != right_replayed, "exactly one is the replay");
    assert_eq!(vouchers_with_code(&url, &code), 1);
}

#[tokio::test]
async fn a_retried_discount_or_flash_sale_is_not_inserted_twice() {
    let Some(url) = database_url() else {
        return;
    };
    let pool = create_pool(&url);
    let now = Utc::now();
    let discount_key = unique("key");
    let discount = CreateDiscountRequest {
        title: unique("discount"),
        discount_type: DiscountType::Percentage,
        value: dec!(10),
        target_product_id: None,
        target_category_id: Some("shoes".to_string()),
        start_time: now,
        end_time: now + Duration::days(1),
    };

    let first = DiscountRepository
        .create(
            &pool,
            discount.clone(),
            None,
            creation("admin-a", &discount_key),
        )
        .await
        .expect("created");
    let again = DiscountRepository
        .create(&pool, discount, None, creation("admin-a", &discount_key))
        .await
        .expect("replayed");
    assert_eq!(first.record.id, again.record.id);
    assert!(again.replayed);

    let sale_key = unique("key");
    let sale = CreateFlashSaleRequest {
        title: "Midnight".to_string(),
        product_id: unique("SKU"),
        flash_price: dec!(9900),
        stock_limit: 5,
        start_time: now,
        end_time: now + Duration::hours(1),
    };
    let first = FlashSaleRepository
        .create(&pool, sale.clone(), None, creation("admin-a", &sale_key))
        .await
        .expect("created");
    let again = FlashSaleRepository
        .create(&pool, sale, None, creation("admin-a", &sale_key))
        .await
        .expect("replayed");
    assert_eq!(first.record.id, again.record.id);
    assert!(again.replayed);

    let reused_for_a_voucher = VoucherRepository
        .create(
            &pool,
            voucher(&unique("CROSS")),
            None,
            creation("admin-a", &sale_key),
        )
        .await;
    assert!(
        matches!(reused_for_a_voucher, Err(AppError::Unprocessable(_))),
        "{reused_for_a_voucher:?}"
    );
}

#[tokio::test]
async fn an_exhausted_voucher_is_not_found_by_get_and_not_offered_by_apply() {
    let Some(url) = database_url() else {
        return;
    };
    let pool = create_pool(&url);
    let promotions = promotions();
    let wanted = unique("SPENT");

    create(&pool, "admin-a", &unique("key"), voucher(&wanted))
        .await
        .expect("created");

    let shown = get_voucher(
        OptionalCaller(None),
        AdminActionTarget::default(),
        (&pool).into(),
        (&promotions).into(),
        &wanted,
    )
    .await
    .expect("a voucher with quota left is found");
    assert!(matches!(shown.into_inner(), VoucherView::Public(_)));

    {
        use kinetix_pricing_service::schema::vouchers::dsl::*;
        diesel::update(vouchers.filter(code.eq(&wanted)))
            .set(used_count.eq(quota))
            .execute(&mut sql(&url))
            .expect("spend the quota");
    }

    let read = get_voucher(
        OptionalCaller(None),
        AdminActionTarget::default(),
        (&pool).into(),
        (&promotions).into(),
        &wanted,
    )
    .await;
    assert!(
        matches!(read, Err(AppError::NotFound(_))),
        "{:?}",
        read.err()
    );

    let applied = apply_voucher(
        OptionalCaller(None),
        AdminActionTarget::default(),
        (&pool).into(),
        (&promotions).into(),
        JsonBody(kinetix_pricing_service::models::ApplyVoucherRequest::new(
            wanted.clone(),
            dec!(100000),
            None,
        )),
    )
    .await
    .expect("apply answers");
    assert!(applied.into_inner().is_none());
}

async fn sale_on(
    pool: &DbPool,
    product: &str,
    flash_price: rust_decimal::Decimal,
    stock_limit: i32,
) -> Uuid {
    return sale_owned_by(pool, product, flash_price, stock_limit, None).await;
}

async fn sale_owned_by(
    pool: &DbPool,
    product: &str,
    flash_price: rust_decimal::Decimal,
    stock_limit: i32,
    owner: Option<&str>,
) -> Uuid {
    let now = Utc::now();
    let created = FlashSaleRepository
        .create(
            pool,
            CreateFlashSaleRequest {
                title: "sale".to_string(),
                product_id: product.to_string(),
                flash_price,
                stock_limit,
                start_time: now - Duration::minutes(5),
                end_time: now + Duration::hours(1),
            },
            owner.map(str::to_string),
            creation("admin-a", &unique("key")),
        )
        .await
        .expect("sale created");

    return created.record.id;
}

fn sell(url: &str, sale: Uuid, sold: i32) {
    use kinetix_pricing_service::schema::flash_sales::dsl::*;

    diesel::update(flash_sales.filter(id.eq(sale)))
        .set(stock_sold.eq(sold))
        .execute(&mut sql(url))
        .expect("record sales");
}

#[tokio::test]
async fn the_flash_sale_lookup_picks_the_cheapest_sale_that_still_has_the_units() {
    let Some(url) = database_url() else {
        return;
    };
    let pool = create_pool(&url);
    let product = unique("SKU");

    let sold_out = sale_on(&pool, &product, dec!(50), 10).await;
    sell(&url, sold_out, 10);
    let roomy = sale_on(&pool, &product, dec!(70), 10).await;
    let small = sale_on(&pool, &product, dec!(60), 2).await;

    let lookup = |units: i32| {
        let pool = &pool;
        let product = &product;
        return async move {
            return FlashSaleRepository
                .find_active_for_product(pool, product, units)
                .await
                .expect("lookup")
                .map(|sale| return sale.id);
        };
    };

    assert_eq!(lookup(1).await, Some(small));
    assert_eq!(lookup(2).await, Some(small));
    assert_eq!(lookup(3).await, Some(roomy));

    sell(&url, roomy, 10);
    sell(&url, small, 2);
    assert_eq!(lookup(1).await, None);
}

#[tokio::test]
async fn two_sales_at_one_price_resolve_to_the_older_every_time() {
    let Some(url) = database_url() else {
        return;
    };
    let pool = create_pool(&url);
    let product = unique("SKU");

    let older = sale_on(&pool, &product, dec!(80), 5).await;
    sale_on(&pool, &product, dec!(80), 5).await;

    for _ in 0..5 {
        let found = FlashSaleRepository
            .find_active_for_product(&pool, &product, 1)
            .await
            .expect("lookup")
            .map(|sale| return sale.id);
        assert_eq!(found, Some(older));
    }
}

#[tokio::test]
async fn the_largest_values_the_checks_accept_fit_the_columns() {
    let Some(url) = database_url() else {
        return;
    };
    let pool = create_pool(&url);
    let code = format!("{}{}", unique("MAX"), "é".repeat(60));
    let mut req = voucher(&code);
    req.title = "é".repeat(255);
    req.value = dec!(9_999_999_999.99);
    req.min_spend = dec!(9_999_999_999.99);
    req.max_discount = Some(dec!(9_999_999_999.99));
    req.quota = i32::MAX;

    let longest_key = format!("{}{}", unique("key"), "k".repeat(215));
    assert_eq!(longest_key.chars().count(), 255);
    let (made, _) = create(&pool, "admin-a", &longest_key, req)
        .await
        .expect("the largest accepted voucher fits its columns");
    assert_eq!(made.value, dec!(9_999_999_999.99));
    assert_eq!(made.code.chars().count(), code.chars().count());

    let earliest =
        DateTime::from_timestamp(EARLIEST_STORABLE_UNIX_SECONDS, 0).expect("representable");
    let discount = CreateDiscountRequest {
        title: "é".repeat(255),
        discount_type: DiscountType::Fixed,
        value: dec!(9_999_999_999.99),
        target_product_id: Some("p".repeat(100)),
        target_category_id: Some("c".repeat(100)),
        start_time: earliest,
        end_time: Utc::now() + Duration::days(1),
    };
    let stored = DiscountRepository
        .create(&pool, discount, None, creation("admin-a", &unique("key")))
        .await
        .expect("the earliest accepted start fits its column");
    assert_eq!(stored.record.start_time, earliest);
}

#[tokio::test]
async fn a_sellers_flash_sale_never_hides_a_platform_sale_from_another_merchants_cart() {
    let Some(url) = database_url() else {
        return;
    };
    let pool = create_pool(&url);
    let product = unique("SKU");

    let sellers = sale_owned_by(&pool, &product, dec!(10), 5, Some("merchant-a")).await;
    let platform = sale_owned_by(&pool, &product, dec!(20), 5, None).await;

    let for_cart = |merchant: Option<&'static str>| {
        let pool = &pool;
        let product = &product;
        return async move {
            return FlashSaleRepository
                .find_active_for_cart(pool, product, 1, merchant)
                .await
                .expect("lookup")
                .map(|sale| return sale.id);
        };
    };

    assert_eq!(for_cart(Some("merchant-a")).await, Some(sellers));
    assert_eq!(for_cart(Some("merchant-b")).await, Some(platform));
    assert_eq!(for_cart(None).await, Some(platform));

    let public = FlashSaleRepository
        .find_active_for_product(&pool, &product, 1)
        .await
        .expect("lookup")
        .map(|sale| return sale.id);
    assert_eq!(public, Some(sellers));
}

fn expire(url: &str, wanted: &str) {
    use kinetix_pricing_service::schema::vouchers::dsl::*;

    diesel::update(vouchers.filter(code.eq(wanted)))
        .set(expires_at.eq(Utc::now() - Duration::minutes(1)))
        .execute(&mut sql(url))
        .expect("expire the voucher");
}

#[tokio::test]
async fn an_expired_voucher_is_not_redeemed_but_a_redemption_already_held_still_replays() {
    let Some(url) = database_url() else {
        return;
    };
    let pool = create_pool(&url);
    let wanted = unique("LATE");
    let mut req = voucher(&wanted);
    req.quota = 5;
    create(&pool, "admin-a", &unique("key"), req)
        .await
        .expect("created");
    let held_order = unique("ORD");

    let first = QuotaRepository::redeem_voucher(&pool, &wanted, &held_order, "buyer", None)
        .await
        .expect("redeemed");
    assert!(matches!(first, QuotaOutcome::Applied { .. }), "{first:?}");

    expire(&url, &wanted);

    let replay = QuotaRepository::redeem_voucher(&pool, &wanted, &held_order, "buyer", None)
        .await
        .expect("replayed");
    assert!(
        matches!(replay, QuotaOutcome::AlreadyDone { .. }),
        "{replay:?}"
    );

    let late = QuotaRepository::redeem_voucher(&pool, &wanted, &unique("ORD"), "buyer", None)
        .await
        .expect("answered");
    assert_eq!(late, QuotaOutcome::Expired);

    let answer = PricingGrpcServer::new(create_pool(&url))
        .redeem_voucher(tonic::Request::new(RedeemVoucherRequest {
            voucher_code: wanted.clone(),
            order_number: unique("ORD"),
            customer_principal_id: "buyer".to_string(),
            ..Default::default()
        }))
        .await
        .expect("RedeemVoucher answers")
        .into_inner();
    assert!(!answer.success);
    assert_eq!(
        answer.error.map(|error| return error.error_code).as_deref(),
        Some("VOUCHER_EXPIRED")
    );
}

#[tokio::test]
async fn a_switched_off_voucher_is_not_found_rather_than_exhausted() {
    let Some(url) = database_url() else {
        return;
    };
    let pool = create_pool(&url);
    let wanted = unique("OFF");
    create(&pool, "admin-a", &unique("key"), voucher(&wanted))
        .await
        .expect("created");

    {
        use kinetix_pricing_service::schema::vouchers::dsl::*;
        diesel::update(vouchers.filter(code.eq(&wanted)))
            .set(active.eq(false))
            .execute(&mut sql(&url))
            .expect("switch off");
    }

    let answer = QuotaRepository::redeem_voucher(&pool, &wanted, &unique("ORD"), "buyer", None)
        .await
        .expect("answered");
    assert_eq!(answer, QuotaOutcome::NotFound);
}

#[tokio::test]
async fn voucher_codes_are_one_code_whatever_their_case() {
    let Some(url) = database_url() else {
        return;
    };
    let pool = create_pool(&url);
    let promotions = promotions();
    let typed = format!("save-{}", Uuid::new_v4()).to_lowercase();
    let stored = typed.to_uppercase();

    let (made, _) = create(&pool, "admin-a", &unique("key"), voucher(&typed))
        .await
        .expect("created");
    assert_eq!(made.code, stored);

    let mixed = format!("S{}", &typed[1..]);
    let shown = get_voucher(
        OptionalCaller(None),
        AdminActionTarget::default(),
        (&pool).into(),
        (&promotions).into(),
        &mixed,
    )
    .await
    .expect("found whatever the case");
    match shown.into_inner() {
        VoucherView::Public(voucher) => assert_eq!(voucher.code, stored),
        VoucherView::Full(voucher) => assert_eq!(voucher.code, stored),
    }

    let again = create(&pool, "admin-b", &unique("key"), voucher(&stored)).await;
    assert!(matches!(again, Err(AppError::Conflict(_))), "{again:?}");

    let redeemed = QuotaRepository::redeem_voucher(&pool, &typed, &unique("ORD"), "buyer", None)
        .await
        .expect("redeemed");
    assert!(
        matches!(redeemed, QuotaOutcome::Applied { .. }),
        "{redeemed:?}"
    );

    let bypass = diesel::sql_query(
        "INSERT INTO vouchers (id, code, title, discount_type, value, min_spend, quota, \
         used_count, active, expires_at) \
         VALUES (gen_random_uuid(), $1, 't', 'FIXED', 1, 0, 1, 0, true, now() + interval '1 day')",
    )
    .bind::<diesel::sql_types::Text, _>(&typed)
    .execute(&mut sql(&url));
    assert!(
        matches!(
            &bypass,
            Err(diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UniqueViolation,
                _
            ))
        ),
        "the database let a second case of one code in: {bypass:?}"
    );
}
