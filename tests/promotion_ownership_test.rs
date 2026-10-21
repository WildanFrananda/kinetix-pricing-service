use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{Duration, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use uuid::Uuid;

use kinetix_pricing_service::clients::{
    MerchantDirectoryPort, MerchantStanding, PeerError, ProductDirectoryPort,
};
use kinetix_pricing_service::error::AppError;
use kinetix_pricing_service::guards::AuthenticatedCaller;
use kinetix_pricing_service::models::{DiscountType, Voucher, VoucherView};
use kinetix_pricing_service::services::{previewed, validate_percentage, PromotionService};
use kinetix_pricing_service::traits::{applies_to_cart, DefaultVoucherEvaluator};

const SELLER: &str = "principal-of-the-seller";
const MERCHANT: &str = "merchant-principal-identity-names";

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

fn catalog(entries: &[(&str, &str)]) -> Products {
    return Products(Ok(entries
        .iter()
        .map(|(sku, owner)| return (sku.to_string(), owner.to_string()))
        .collect()));
}

fn unreachable(service: &str) -> PeerError {
    return PeerError::Unavailable(format!("{service} did not answer"));
}

fn promotions(merchants: Merchants, products: Products) -> PromotionService {
    return PromotionService::new(Arc::new(merchants), Arc::new(products));
}

#[tokio::test]
async fn an_admin_creates_a_platform_promotion_with_no_product_without_asking_anyone() {
    let service = promotions(
        Merchants(Err(unreachable("identity"))),
        Products(Err(unreachable("catalog"))),
    );

    let owner = service.owner_for("admin-principal", "admin", None).await;

    assert!(matches!(owner, Ok(None)));
}

#[tokio::test]
async fn an_admin_promotes_a_product_catalog_knows_as_a_platform_promotion() {
    let service = promotions(
        Merchants(Err(unreachable("identity"))),
        catalog(&[("SKU-1", MERCHANT)]),
    );

    let owner = service
        .owner_for("admin-principal", "admin", Some("SKU-1"))
        .await;

    assert!(matches!(owner, Ok(None)), "{owner:?}");
}

#[tokio::test]
async fn an_admin_cannot_promote_a_product_catalog_does_not_know() {
    let service = promotions(Merchants(Err(unreachable("identity"))), catalog(&[]));

    let owner = service
        .owner_for("admin-principal", "admin", Some("SKU-NOWHERE"))
        .await;

    assert!(matches!(owner, Err(AppError::NotFound(_))), "{owner:?}");
}

#[tokio::test]
async fn an_admin_promotion_of_a_product_waits_for_catalog_rather_than_guessing() {
    let service = promotions(
        Merchants(Err(unreachable("identity"))),
        Products(Err(unreachable("catalog"))),
    );

    let owner = service
        .owner_for("admin-principal", "admin", Some("SKU-1"))
        .await;

    assert!(matches!(owner, Err(AppError::Unavailable(_))), "{owner:?}");
}

#[tokio::test]
async fn a_product_id_catalog_refuses_is_a_bad_request_not_an_outage() {
    let service = promotions(
        trading(true),
        Products(Err(PeerError::Refused(
            "catalog refused GetProduct: sku is required".to_string(),
        ))),
    );

    for role in ["admin", "seller"] {
        let owner = service.owner_for(SELLER, role, Some("")).await;
        assert!(
            matches!(&owner, Err(AppError::BadRequest(message)) if message.contains("sku is required")),
            "role {role}: {owner:?}"
        );
    }
}

#[tokio::test]
async fn an_account_identity_refuses_to_look_up_is_refused_not_an_outage() {
    let service = promotions(
        Merchants(Err(PeerError::Refused(
            "identity refused GetMerchantInfo: bad principal".to_string(),
        ))),
        catalog(&[]),
    );

    let owner = service.owner_for(SELLER, "seller", None).await;

    assert!(matches!(owner, Err(AppError::Forbidden(_))), "{owner:?}");
}

#[tokio::test]
async fn a_seller_owns_what_they_create_under_the_key_identity_names() {
    let service = promotions(trading(true), catalog(&[]));

    let owner = service.owner_for(SELLER, "seller", None).await;

    assert_eq!(owner.ok().flatten().as_deref(), Some(MERCHANT));
}

#[tokio::test]
async fn a_seller_may_promote_their_own_product() {
    let service = promotions(trading(true), catalog(&[("SKU-OWN", MERCHANT)]));

    let owner = service.owner_for(SELLER, "seller", Some("SKU-OWN")).await;

    assert_eq!(owner.ok().flatten().as_deref(), Some(MERCHANT));
}

#[tokio::test]
async fn a_seller_cannot_promote_another_merchants_product() {
    let service = promotions(trading(true), catalog(&[("SKU-THEIRS", "someone-else")]));

    let owner = service
        .owner_for(SELLER, "seller", Some("SKU-THEIRS"))
        .await;

    assert!(matches!(owner, Err(AppError::Forbidden(_))));
}

#[tokio::test]
async fn a_product_catalog_does_not_know_is_not_found() {
    let service = promotions(trading(true), catalog(&[]));

    let owner = service
        .owner_for(SELLER, "seller", Some("SKU-NOWHERE"))
        .await;

    assert!(matches!(owner, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn a_merchant_identity_will_not_let_trade_is_refused() {
    let service = promotions(trading(false), catalog(&[("SKU-OWN", MERCHANT)]));

    for product in [None, Some("SKU-OWN")] {
        let owner = service.owner_for(SELLER, "seller", product).await;
        assert!(
            matches!(owner, Err(AppError::Forbidden(_))),
            "product {product:?}"
        );
    }
}

#[tokio::test]
async fn an_account_identity_knows_no_merchant_for_is_refused() {
    let service = promotions(Merchants(Ok(None)), catalog(&[]));

    let owner = service.owner_for(SELLER, "seller", None).await;

    assert!(matches!(owner, Err(AppError::Forbidden(_))));
}

#[tokio::test]
async fn identity_not_answering_is_unavailable_not_a_refusal() {
    let service = promotions(Merchants(Err(unreachable("identity"))), catalog(&[]));

    let owner = service.owner_for(SELLER, "seller", None).await;

    assert!(matches!(owner, Err(AppError::Unavailable(_))));
}

#[tokio::test]
async fn catalog_not_answering_is_unavailable_not_a_refusal() {
    let service = promotions(trading(true), Products(Err(unreachable("catalog"))));

    let owner = service.owner_for(SELLER, "seller", Some("SKU-1")).await;

    assert!(matches!(owner, Err(AppError::Unavailable(_))));
}

#[tokio::test]
async fn no_other_role_may_create_a_promotion() {
    let service = promotions(trading(true), catalog(&[]));

    for role in ["customer", "courier", ""] {
        let owner = service.owner_for(SELLER, role, None).await;
        assert!(
            matches!(owner, Err(AppError::Forbidden(_))),
            "role {role:?}"
        );
    }
}

#[test]
fn a_percentage_above_one_hundred_is_refused() {
    assert!(validate_percentage(&DiscountType::Percentage, dec!(100)).is_ok());
    assert!(matches!(
        validate_percentage(&DiscountType::Percentage, dec!(100.01)),
        Err(AppError::BadRequest(_))
    ));
    assert!(validate_percentage(&DiscountType::Fixed, dec!(250000)).is_ok());
}

#[test]
fn a_promotion_reaches_a_cart_only_when_it_is_the_platforms_or_the_carts_own() {
    assert!(applies_to_cart(None, None));
    assert!(applies_to_cart(None, Some(MERCHANT)));
    assert!(applies_to_cart(Some(MERCHANT), Some(MERCHANT)));
    assert!(!applies_to_cart(Some(MERCHANT), Some("someone-else")));
    assert!(!applies_to_cart(Some(MERCHANT), None));
}

fn voucher_owned_by(owner: Option<&str>, min_spend: Decimal) -> Voucher {
    let now = Utc::now();
    return Voucher {
        id: Uuid::new_v4(),
        code: "SHOP10".to_string(),
        title: "preview".to_string(),
        discount_type: "PERCENTAGE".to_string(),
        value: dec!(10),
        min_spend,
        max_discount: None,
        quota: 10,
        used_count: 0,
        active: true,
        expires_at: now + Duration::days(1),
        created_at: now,
        updated_at: now,
        merchant_principal_id: owner.map(|m| return m.to_string()),
    };
}

#[test]
fn a_platform_voucher_previews_for_any_cart() {
    for cart in [None, Some(MERCHANT)] {
        let shown = previewed(
            voucher_owned_by(None, dec!(0)),
            &DefaultVoucherEvaluator,
            dec!(100000),
            cart,
        );
        assert!(shown.is_some(), "cart {cart:?}");
    }
}

#[test]
fn a_sellers_voucher_previews_only_for_that_merchants_cart() {
    let preview = |cart: Option<&str>| {
        return previewed(
            voucher_owned_by(Some(MERCHANT), dec!(0)),
            &DefaultVoucherEvaluator,
            dec!(100000),
            cart,
        );
    };

    assert!(preview(Some(MERCHANT)).is_some());
    assert!(preview(Some("another-merchant")).is_none());
    assert!(preview(None).is_none());
}

#[test]
fn a_voucher_below_its_minimum_spend_previews_as_nothing_even_for_its_merchant() {
    let shown = previewed(
        voucher_owned_by(Some(MERCHANT), dec!(500000)),
        &DefaultVoucherEvaluator,
        dec!(100000),
        Some(MERCHANT),
    );
    assert!(shown.is_none());
}

fn viewer(principal_id: &str, role: &str) -> AuthenticatedCaller {
    return AuthenticatedCaller {
        principal_id: principal_id.to_string(),
        email: format!("{principal_id}@kinetix.test"),
        role: role.to_string(),
    };
}

fn fields_of(view: VoucherView) -> Vec<String> {
    let json = serde_json::to_value(view).expect("the view did not serialize");
    let mut fields: Vec<String> = json
        .as_object()
        .expect("the view is not an object")
        .keys()
        .cloned()
        .collect();
    fields.sort();
    return fields;
}

const BUYER_FIELDS: [&str; 8] = [
    "code",
    "discount_type",
    "expires_at",
    "max_discount",
    "merchant_principal_id",
    "min_spend",
    "title",
    "value",
];

#[tokio::test]
async fn an_anonymous_reader_sees_only_what_a_buyer_needs() {
    let service = promotions(Merchants(Err(unreachable("identity"))), catalog(&[]));

    let view = service
        .voucher_view(voucher_owned_by(Some(MERCHANT), dec!(0)), None)
        .await
        .expect("an anonymous read needs nobody");

    assert_eq!(fields_of(view), BUYER_FIELDS);
}

#[tokio::test]
async fn a_customer_sees_only_what_a_buyer_needs_without_identity_being_asked() {
    let service = promotions(Merchants(Err(unreachable("identity"))), catalog(&[]));

    let view = service
        .voucher_view(
            voucher_owned_by(Some(MERCHANT), dec!(0)),
            Some(&viewer("buyer", "customer")),
        )
        .await
        .expect("a customer read needs nobody");

    assert_eq!(fields_of(view), BUYER_FIELDS);
}

#[tokio::test]
async fn an_admin_sees_the_quota_and_how_much_of_it_is_used() {
    let service = promotions(Merchants(Err(unreachable("identity"))), catalog(&[]));

    let view = service
        .voucher_view(
            voucher_owned_by(Some(MERCHANT), dec!(0)),
            Some(&viewer("admin", "admin")),
        )
        .await
        .expect("an admin read needs nobody");

    let fields = fields_of(view);
    assert!(fields.contains(&"quota".to_string()), "{fields:?}");
    assert!(fields.contains(&"used_count".to_string()), "{fields:?}");
}

#[tokio::test]
async fn the_seller_who_owns_a_voucher_sees_its_quota() {
    let service = promotions(trading(true), catalog(&[]));

    let view = service
        .voucher_view(
            voucher_owned_by(Some(MERCHANT), dec!(0)),
            Some(&viewer(SELLER, "seller")),
        )
        .await
        .expect("identity answered");

    assert!(matches!(view, VoucherView::Full(_)), "{view:?}");
}

#[tokio::test]
async fn another_seller_sees_only_what_a_buyer_needs() {
    let service = promotions(trading(true), catalog(&[]));

    let view = service
        .voucher_view(
            voucher_owned_by(Some("someone-else"), dec!(0)),
            Some(&viewer(SELLER, "seller")),
        )
        .await
        .expect("identity answered");

    assert_eq!(fields_of(view), BUYER_FIELDS);
}

#[tokio::test]
async fn a_seller_reading_a_platform_voucher_sees_the_buyers_view_without_asking_identity() {
    let service = promotions(Merchants(Err(unreachable("identity"))), catalog(&[]));

    let view = service
        .voucher_view(
            voucher_owned_by(None, dec!(0)),
            Some(&viewer(SELLER, "seller")),
        )
        .await
        .expect("a platform voucher has no merchant to compare");

    assert_eq!(fields_of(view), BUYER_FIELDS);
}

#[tokio::test]
async fn a_seller_reading_a_merchant_voucher_while_identity_is_down_is_unavailable() {
    let service = promotions(Merchants(Err(unreachable("identity"))), catalog(&[]));

    let view = service
        .voucher_view(
            voucher_owned_by(Some(MERCHANT), dec!(0)),
            Some(&viewer(SELLER, "seller")),
        )
        .await;

    assert!(matches!(view, Err(AppError::Unavailable(_))), "{view:?}");
}
