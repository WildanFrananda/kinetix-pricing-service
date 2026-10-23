use async_trait::async_trait;
use chrono::{Duration, Utc};
use proptest::prelude::*;
use rust_decimal::Decimal;
use uuid::Uuid;

use kinetix_pricing_service::db::create_pool;
use kinetix_pricing_service::error::AppError;
use kinetix_pricing_service::models::{
    CalculatePriceRequest, CreateDiscountRequest, CreateFlashSaleRequest, CreateOutcome,
    CreateVoucherRequest, Discount, FlashSale, IdempotentCreation, PriceItemRequest, ShippingRate,
    Voucher,
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

const CENTS_RANGE: i64 = 2_000_000_000_000;

fn money() -> impl Strategy<Value = Decimal> {
    return prop_oneof![
        (-CENTS_RANGE..CENTS_RANGE).prop_map(|cents| return Decimal::new(cents, 2)),
        (-20_000_i64..20_000).prop_map(|cents| return Decimal::new(cents, 2)),
        Just(Decimal::ZERO),
        Just(Decimal::new(10_000, 2)),
    ];
}

fn kind() -> impl Strategy<Value = String> {
    return prop_oneof![
        4 => Just("PERCENTAGE".to_string()),
        4 => Just("FIXED".to_string()),
        1 => Just("percentage".to_string()),
        1 => Just("SHIPPING".to_string()),
        1 => Just(String::new()),
    ];
}

fn hours_to_expiry() -> impl Strategy<Value = i64> {
    return prop_oneof![4 => 1_i64..48, 1 => -48_i64..1];
}

prop_compose! {
    fn stored_voucher()(
        discount_type in kind(),
        value in money(),
        min_spend in money(),
        max_discount in proptest::option::weighted(0.7, money()),
        quota in -1_i32..6,
        used_count in -1_i32..3,
        active in proptest::bool::weighted(0.85),
        expires_in_hours in hours_to_expiry(),
    ) -> Voucher {
        let now = Utc::now();
        return Voucher {
            id: Uuid::new_v4(),
            code: "PROPERTY".to_string(),
            title: "stored".to_string(),
            discount_type,
            value,
            min_spend,
            max_discount,
            quota,
            used_count,
            active,
            expires_at: now + Duration::hours(expires_in_hours),
            created_at: now,
            updated_at: now,
            merchant_principal_id: None,
        };
    }
}

prop_compose! {
    fn stored_discount()(discount_type in kind(), value in money()) -> Discount {
        let now = Utc::now();
        return Discount {
            id: Uuid::new_v4(),
            title: "stored".to_string(),
            discount_type,
            value,
            target_product_id: None,
            target_category_id: None,
            active: true,
            start_time: now - Duration::days(1),
            end_time: now + Duration::days(1),
            created_at: now,
            updated_at: now,
            merchant_principal_id: None,
        };
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2048))]

    #[test]
    fn a_voucher_discount_is_never_below_zero_nor_above_the_subtotal(
        voucher in stored_voucher(),
        subtotal in money(),
    ) {
        let discount = DefaultVoucherEvaluator.calculate_discount(&voucher, subtotal);

        prop_assert!(discount >= Decimal::ZERO, "discount {discount} on {subtotal}");
        prop_assert!(discount <= subtotal.max(Decimal::ZERO), "discount {discount} on {subtotal}");
    }

    #[test]
    fn a_voucher_discount_never_exceeds_a_positive_cap(
        voucher in stored_voucher(),
        subtotal in money(),
    ) {
        let discount = DefaultVoucherEvaluator.calculate_discount(&voucher, subtotal);

        if let Some(cap) = voucher.max_discount.filter(|cap| return *cap > Decimal::ZERO) {
            prop_assert!(discount <= cap, "discount {discount} above cap {cap}");
        }
    }

    #[test]
    fn an_ineligible_voucher_takes_nothing_off(
        voucher in stored_voucher(),
        subtotal in money(),
    ) {
        if !DefaultVoucherEvaluator.is_eligible(&voucher, subtotal) {
            prop_assert_eq!(
                DefaultVoucherEvaluator.calculate_discount(&voucher, subtotal),
                Decimal::ZERO
            );
        }
    }

    #[test]
    fn a_discount_saving_is_never_below_zero_nor_above_the_price(
        discount in stored_discount(),
        base_price in money(),
    ) {
        let savings = DefaultDiscountEvaluator.calculate_savings(&discount, base_price);

        prop_assert!(savings >= Decimal::ZERO, "savings {savings} on {base_price}");
        prop_assert!(savings <= base_price.max(Decimal::ZERO), "savings {savings} on {base_price}");
    }
}

struct Discounts(Vec<Discount>);
struct Vouchers(Option<Voucher>);
struct FlashSales(Option<FlashSale>);
struct NoShippingRates;

#[async_trait]
impl DiscountRepositoryPort for Discounts {
    async fn find_all_active(&self, _pool: &DbPool) -> Result<Vec<Discount>, AppError> {
        return Ok(self.0.clone());
    }

    async fn earlier_creation(
        &self,
        _pool: &DbPool,
        _req: &CreateDiscountRequest,
        _creation: &IdempotentCreation,
    ) -> Result<Option<Discount>, AppError> {
        unimplemented!("not used by these tests");
    }

    async fn create(
        &self,
        _pool: &DbPool,
        _req: CreateDiscountRequest,
        _owner: Option<String>,
        _creation: IdempotentCreation,
    ) -> Result<CreateOutcome<Discount>, AppError> {
        unimplemented!("not used by these tests");
    }
}

#[async_trait]
impl VoucherRepositoryPort for Vouchers {
    async fn find_by_code(&self, _pool: &DbPool, _code: &str) -> Result<Option<Voucher>, AppError> {
        return Ok(self.0.clone());
    }

    async fn earlier_creation(
        &self,
        _pool: &DbPool,
        _req: &CreateVoucherRequest,
        _creation: &IdempotentCreation,
    ) -> Result<Option<Voucher>, AppError> {
        unimplemented!("not used by these tests");
    }

    async fn create(
        &self,
        _pool: &DbPool,
        _req: CreateVoucherRequest,
        _owner: Option<String>,
        _creation: IdempotentCreation,
    ) -> Result<CreateOutcome<Voucher>, AppError> {
        unimplemented!("not used by these tests");
    }
}

#[async_trait]
impl FlashSaleRepositoryPort for FlashSales {
    async fn find_active_for_product(
        &self,
        _pool: &DbPool,
        _product_id: &str,
        _units: i32,
    ) -> Result<Option<FlashSale>, AppError> {
        return Ok(self.0.clone());
    }

    async fn find_active_for_cart(
        &self,
        _pool: &DbPool,
        _product_id: &str,
        _units: i32,
        _cart_merchant: Option<&str>,
    ) -> Result<Option<FlashSale>, AppError> {
        return Ok(self.0.clone());
    }

    async fn earlier_creation(
        &self,
        _pool: &DbPool,
        _req: &CreateFlashSaleRequest,
        _creation: &IdempotentCreation,
    ) -> Result<Option<FlashSale>, AppError> {
        unimplemented!("not used by these tests");
    }

    async fn create(
        &self,
        _pool: &DbPool,
        _req: CreateFlashSaleRequest,
        _owner: Option<String>,
        _creation: IdempotentCreation,
    ) -> Result<CreateOutcome<FlashSale>, AppError> {
        unimplemented!("not used by these tests");
    }
}

#[async_trait]
impl ShippingRateRepositoryPort for NoShippingRates {
    async fn find_by_tier(
        &self,
        _pool: &DbPool,
        _tier: &str,
    ) -> Result<Option<ShippingRate>, AppError> {
        return Ok(None);
    }
}

fn flash_sale_at(flash_price: Decimal) -> FlashSale {
    let now = Utc::now();
    return FlashSale {
        id: Uuid::new_v4(),
        title: "stored".to_string(),
        product_id: "SKU-1".to_string(),
        flash_price,
        stock_limit: 10,
        stock_sold: 0,
        active: true,
        start_time: now - Duration::hours(1),
        end_time: now + Duration::hours(1),
        created_at: now,
        updated_at: now,
        merchant_principal_id: None,
    };
}

#[test]
fn no_stored_promotion_makes_the_bill_larger_than_the_goods_and_the_shipping() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("a test runtime");
    let pool = runtime.block_on(async {
        return create_pool("postgres://unused:unused@127.0.0.1:1/unused");
    });

    proptest!(|(
        discounts in proptest::collection::vec(stored_discount(), 0..3),
        voucher in proptest::option::of(stored_voucher()),
        flash_price in proptest::option::of(money()),
        base_cents in 0_i64..CENTS_RANGE,
        quantity in 1_i32..5,
        shipping_cents in 0_i64..10_000_000,
    )| {
        let base_price = Decimal::new(base_cents, 2);
        let shipping = Decimal::new(shipping_cents, 2);
        let goods = base_price * Decimal::from(quantity);
        let service = PricingService::new(
            Discounts(discounts),
            Vouchers(voucher),
            FlashSales(flash_price.map(flash_sale_at)),
            NoShippingRates,
            Box::new(DefaultDiscountEvaluator),
            Box::new(DefaultVoucherEvaluator),
        );
        let request = CalculatePriceRequest {
            items: vec![PriceItemRequest {
                product_id: "SKU-1".to_string(),
                category_id: None,
                base_price,
                quantity,
            }],
            voucher_code: Some("PROPERTY".to_string()),
            base_shipping_fee: Some(shipping),
            shipping: None,
            merchant_principal_id: None,
        };

        let priced = runtime
            .block_on(service.calculate_price(&pool, request))
            .expect("calculate_price should succeed");

        prop_assert!(priced.subtotal >= Decimal::ZERO, "subtotal {}", priced.subtotal);
        prop_assert!(priced.subtotal <= goods, "subtotal {} above goods {goods}", priced.subtotal);
        prop_assert!(priced.voucher_discount >= Decimal::ZERO);
        prop_assert!(priced.voucher_discount <= priced.subtotal);
        prop_assert!(priced.final_total >= shipping, "total {} below shipping", priced.final_total);
        prop_assert!(
            priced.final_total <= goods + shipping,
            "total {} above goods {goods} and shipping {shipping}",
            priced.final_total
        );
        prop_assert_eq!(priced.final_total, priced.subtotal - priced.voucher_discount + shipping);
    });
}
