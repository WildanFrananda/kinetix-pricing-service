use std::collections::HashMap;
use std::sync::{Arc, Once};

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use diesel::{Connection, PgConnection};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use rust_decimal_macros::dec;
use uuid::Uuid;

use kinetix_pricing_service::clients::{
    MerchantDirectoryPort, MerchantStanding, PeerError, ProductDirectoryPort,
};
use kinetix_pricing_service::db::create_pool;
use kinetix_pricing_service::error::AppError;
use kinetix_pricing_service::guards::{
    AdminActionTarget, AdminOrMerchantGuard, IdempotencyKey, JsonBody,
};
use kinetix_pricing_service::models::{
    CreateDiscountRequest, CreateFlashSaleRequest, CreateVoucherRequest, DiscountType,
    IdempotentCreation,
};
use kinetix_pricing_service::repositories::{
    DiscountRepository, DiscountRepositoryPort, FlashSaleRepository, FlashSaleRepositoryPort,
    VoucherRepository, VoucherRepositoryPort,
};
use kinetix_pricing_service::routes::discount_routes::create_discount;
use kinetix_pricing_service::routes::flash_sale_routes::create_flash_sale;
use kinetix_pricing_service::routes::voucher_routes::create_voucher;
use kinetix_pricing_service::services::PromotionService;
use kinetix_pricing_service::DbPool;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");
const DATABASE_VARIABLE: &str = "KINETIX_TEST_POSTGRES";
const MERCHANT: &str = "merchant-principal-identity-names";
const PRODUCT: &str = "SKU-OF-THE-MERCHANT";

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

struct Merchants(Result<Option<MerchantStanding>, PeerError>);
struct Products(Result<HashMap<String, String>, PeerError>);

#[async_trait]
impl MerchantDirectoryPort for Merchants {
    async fn standing_of(
        &self,
        _principal_id: &str,
    ) -> Result<Option<MerchantStanding>, PeerError> {
        return self.0.clone();
    }
}

#[async_trait]
impl ProductDirectoryPort for Products {
    async fn owner_of(&self, sku: &str) -> Result<Option<String>, PeerError> {
        return self.0.clone().map(|owners| return owners.get(sku).cloned());
    }
}

fn trading(may_sell: bool) -> Merchants {
    return Merchants(Ok(Some(MerchantStanding {
        merchant_principal_id: MERCHANT.to_string(),
        may_sell,
    })));
}

fn catalog_knows_the_product() -> Products {
    return Products(Ok(HashMap::from([(
        PRODUCT.to_string(),
        MERCHANT.to_string(),
    )])));
}

fn catalog_forgot_the_product() -> Products {
    return Products(Ok(HashMap::new()));
}

fn down(service: &str) -> PeerError {
    return PeerError::Unavailable(format!("{service} did not answer"));
}

fn promotions(merchants: Merchants, products: Products) -> PromotionService {
    return PromotionService::new(Arc::new(merchants), Arc::new(products));
}

fn everyone_answers() -> PromotionService {
    return promotions(trading(true), catalog_knows_the_product());
}

fn caller(principal_id: &str, role: &str) -> AdminOrMerchantGuard {
    return AdminOrMerchantGuard {
        principal_id: principal_id.to_string(),
        role: role.to_string(),
    };
}

fn unique(prefix: &str) -> String {
    return format!("{prefix}-{}", Uuid::new_v4()).to_uppercase();
}

fn discount(end_time: DateTime<Utc>) -> CreateDiscountRequest {
    return CreateDiscountRequest {
        title: "Ten percent off".to_string(),
        discount_type: DiscountType::Percentage,
        value: dec!(10),
        target_product_id: Some(PRODUCT.to_string()),
        target_category_id: None,
        start_time: end_time - Duration::days(2),
        end_time,
    };
}

fn flash_sale(end_time: DateTime<Utc>) -> CreateFlashSaleRequest {
    return CreateFlashSaleRequest {
        title: "Midnight sale".to_string(),
        product_id: PRODUCT.to_string(),
        flash_price: dec!(5000),
        stock_limit: 3,
        start_time: end_time - Duration::days(2),
        end_time,
    };
}

fn voucher(expires_at: DateTime<Utc>) -> CreateVoucherRequest {
    return CreateVoucherRequest {
        code: unique("REPLAY"),
        title: "Ten off".to_string(),
        discount_type: DiscountType::Fixed,
        value: dec!(10000),
        min_spend: dec!(0),
        max_discount: None,
        quota: 1,
        expires_at,
    };
}

fn later() -> DateTime<Utc> {
    return Utc::now() + Duration::days(1);
}

fn earlier() -> DateTime<Utc> {
    return Utc::now() - Duration::hours(1);
}

async fn post_discount(
    pool: &DbPool,
    promotions: &PromotionService,
    who: &AdminOrMerchantGuard,
    key: &str,
    req: CreateDiscountRequest,
) -> Result<(Uuid, bool), AppError> {
    let created = create_discount(
        who.clone(),
        AdminActionTarget::default(),
        IdempotencyKey(key.to_string()),
        pool.into(),
        promotions.into(),
        JsonBody(req),
    )
    .await?;

    return Ok((created.body.into_inner().id, created.replayed));
}

async fn post_flash_sale(
    pool: &DbPool,
    promotions: &PromotionService,
    who: &AdminOrMerchantGuard,
    key: &str,
    req: CreateFlashSaleRequest,
) -> Result<(Uuid, bool), AppError> {
    let created = create_flash_sale(
        who.clone(),
        AdminActionTarget::default(),
        IdempotencyKey(key.to_string()),
        pool.into(),
        promotions.into(),
        JsonBody(req),
    )
    .await?;

    return Ok((created.body.into_inner().id, created.replayed));
}

async fn post_voucher(
    pool: &DbPool,
    promotions: &PromotionService,
    who: &AdminOrMerchantGuard,
    key: &str,
    req: CreateVoucherRequest,
) -> Result<(Uuid, bool), AppError> {
    let created = create_voucher(
        who.clone(),
        AdminActionTarget::default(),
        IdempotencyKey(key.to_string()),
        pool.into(),
        promotions.into(),
        JsonBody(req),
    )
    .await?;

    return Ok((created.body.into_inner().id, created.replayed));
}

fn creation(who: &AdminOrMerchantGuard, key: &str) -> IdempotentCreation {
    return IdempotentCreation {
        principal_id: who.principal_id.clone(),
        idempotency_key: key.to_string(),
    };
}

fn assert_replays(retry: Result<(Uuid, bool), AppError>, first: Uuid, kind: &str) {
    match retry {
        Ok((id, replayed)) => {
            assert_eq!(id, first, "{kind}: the retry answered another promotion");
            assert!(replayed, "{kind}: the retry did not say it was a replay");
        }
        Err(error) => panic!("{kind}: the retry of a create that succeeded was refused: {error:?}"),
    }
    return;
}

#[tokio::test]
async fn a_retry_after_the_window_closed_replays_what_the_first_attempt_created() {
    let Some(url) = database_url() else {
        return;
    };
    let pool = create_pool(&url);
    let admin = caller(&unique("admin"), "admin");
    let service = everyone_answers();

    let (discount_req, flash_req, voucher_req) = (
        discount(earlier()),
        flash_sale(earlier()),
        voucher(earlier()),
    );
    let keys = [unique("key"), unique("key"), unique("key")];

    let first_discount = DiscountRepository
        .create(
            &pool,
            discount_req.clone(),
            None,
            creation(&admin, &keys[0]),
        )
        .await
        .expect("the discount was stored while its window was open");
    let first_flash = FlashSaleRepository
        .create(&pool, flash_req.clone(), None, creation(&admin, &keys[1]))
        .await
        .expect("the flash sale was stored while its window was open");
    let first_voucher = VoucherRepository
        .create(&pool, voucher_req.clone(), None, creation(&admin, &keys[2]))
        .await
        .expect("the voucher was stored before it expired");

    let retried_discount = post_discount(&pool, &service, &admin, &keys[0], discount_req).await;
    let retried_flash = post_flash_sale(&pool, &service, &admin, &keys[1], flash_req).await;
    let retried_voucher = post_voucher(&pool, &service, &admin, &keys[2], voucher_req).await;

    assert_replays(retried_discount, first_discount.record.id, "discount");
    assert_replays(retried_flash, first_flash.record.id, "flash sale");
    assert_replays(retried_voucher, first_voucher.record.id, "voucher");
}

#[tokio::test]
async fn a_retry_after_identity_stopped_the_seller_trading_replays_what_was_created() {
    let Some(url) = database_url() else {
        return;
    };
    let pool = create_pool(&url);
    let seller = caller(&unique("seller"), "seller");
    let before = everyone_answers();
    let after = promotions(trading(false), catalog_knows_the_product());
    let (discount_req, flash_req, voucher_req) =
        (discount(later()), flash_sale(later()), voucher(later()));
    let keys = [unique("key"), unique("key"), unique("key")];

    let (first_discount, _) =
        post_discount(&pool, &before, &seller, &keys[0], discount_req.clone())
            .await
            .expect("a trading seller creates a discount");
    let (first_flash, _) = post_flash_sale(&pool, &before, &seller, &keys[1], flash_req.clone())
        .await
        .expect("a trading seller creates a flash sale");
    let (first_voucher, _) = post_voucher(&pool, &before, &seller, &keys[2], voucher_req.clone())
        .await
        .expect("a trading seller creates a voucher");

    assert_replays(
        post_discount(&pool, &after, &seller, &keys[0], discount_req).await,
        first_discount,
        "discount",
    );
    assert_replays(
        post_flash_sale(&pool, &after, &seller, &keys[1], flash_req).await,
        first_flash,
        "flash sale",
    );
    assert_replays(
        post_voucher(&pool, &after, &seller, &keys[2], voucher_req).await,
        first_voucher,
        "voucher",
    );
}

#[tokio::test]
async fn a_retry_after_catalog_forgot_the_product_replays_what_was_created() {
    let Some(url) = database_url() else {
        return;
    };
    let pool = create_pool(&url);
    let before = everyone_answers();
    let after = promotions(trading(true), catalog_forgot_the_product());

    for who in [
        caller(&unique("admin"), "admin"),
        caller(&unique("seller"), "seller"),
    ] {
        let (discount_req, flash_req) = (discount(later()), flash_sale(later()));
        let keys = [unique("key"), unique("key")];

        let (first_discount, _) =
            post_discount(&pool, &before, &who, &keys[0], discount_req.clone())
                .await
                .expect("a discount of a product catalog knows is created");
        let (first_flash, _) = post_flash_sale(&pool, &before, &who, &keys[1], flash_req.clone())
            .await
            .expect("a flash sale of a product catalog knows is created");

        assert_replays(
            post_discount(&pool, &after, &who, &keys[0], discount_req).await,
            first_discount,
            &format!("{} discount", who.role),
        );
        assert_replays(
            post_flash_sale(&pool, &after, &who, &keys[1], flash_req).await,
            first_flash,
            &format!("{} flash sale", who.role),
        );
    }
}

#[tokio::test]
async fn a_retry_while_identity_and_catalog_are_down_replays_without_asking_them() {
    let Some(url) = database_url() else {
        return;
    };
    let pool = create_pool(&url);
    let before = everyone_answers();
    let after = promotions(
        Merchants(Err(down("identity"))),
        Products(Err(down("catalog"))),
    );

    for who in [
        caller(&unique("admin"), "admin"),
        caller(&unique("seller"), "seller"),
    ] {
        let (discount_req, flash_req, voucher_req) =
            (discount(later()), flash_sale(later()), voucher(later()));
        let keys = [unique("key"), unique("key"), unique("key")];

        let (first_discount, _) =
            post_discount(&pool, &before, &who, &keys[0], discount_req.clone())
                .await
                .expect("a discount is created while everyone answers");
        let (first_flash, _) = post_flash_sale(&pool, &before, &who, &keys[1], flash_req.clone())
            .await
            .expect("a flash sale is created while everyone answers");
        let (first_voucher, _) = post_voucher(&pool, &before, &who, &keys[2], voucher_req.clone())
            .await
            .expect("a voucher is created while everyone answers");

        assert_replays(
            post_discount(&pool, &after, &who, &keys[0], discount_req).await,
            first_discount,
            &format!("{} discount", who.role),
        );
        assert_replays(
            post_flash_sale(&pool, &after, &who, &keys[1], flash_req).await,
            first_flash,
            &format!("{} flash sale", who.role),
        );
        assert_replays(
            post_voucher(&pool, &after, &who, &keys[2], voucher_req).await,
            first_voucher,
            &format!("{} voucher", who.role),
        );
    }
}

#[tokio::test]
async fn a_new_key_is_checked_before_identity_or_catalog_and_still_needs_a_seller_who_may_trade() {
    let Some(url) = database_url() else {
        return;
    };
    let pool = create_pool(&url);
    let admin = caller(&unique("admin"), "admin");
    let seller = caller(&unique("seller"), "seller");
    let unreachable = promotions(
        Merchants(Err(down("identity"))),
        Products(Err(down("catalog"))),
    );
    let stopped = promotions(trading(false), catalog_knows_the_product());

    let closed = post_discount(
        &pool,
        &unreachable,
        &admin,
        &unique("key"),
        discount(earlier()),
    )
    .await;
    let over = post_flash_sale(
        &pool,
        &unreachable,
        &seller,
        &unique("key"),
        flash_sale(earlier()),
    )
    .await;
    let expired = post_voucher(
        &pool,
        &unreachable,
        &seller,
        &unique("key"),
        voucher(earlier()),
    )
    .await;
    let not_trading = post_flash_sale(
        &pool,
        &stopped,
        &seller,
        &unique("key"),
        flash_sale(later()),
    )
    .await;

    for refused in [closed, over, expired] {
        assert!(
            matches!(refused, Err(AppError::BadRequest(_))),
            "{refused:?}"
        );
    }
    assert!(
        matches!(not_trading, Err(AppError::Forbidden(_))),
        "{not_trading:?}"
    );
}

#[tokio::test]
async fn a_key_spent_on_another_request_is_422_even_when_the_new_body_would_be_refused() {
    let Some(url) = database_url() else {
        return;
    };
    let pool = create_pool(&url);
    let admin = caller(&unique("admin"), "admin");
    let key = unique("key");

    post_discount(&pool, &everyone_answers(), &admin, &key, discount(later()))
        .await
        .expect("the first discount is created");

    let reused = post_discount(
        &pool,
        &everyone_answers(),
        &admin,
        &key,
        discount(earlier()),
    )
    .await;

    assert!(
        matches!(reused, Err(AppError::Unprocessable(_))),
        "{reused:?}"
    );
}
