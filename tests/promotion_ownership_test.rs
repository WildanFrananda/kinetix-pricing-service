use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use rust_decimal_macros::dec;

use kinetix_pricing_service::clients::{
    MerchantDirectoryPort, MerchantStanding, PeerUnavailable, ProductDirectoryPort,
};
use kinetix_pricing_service::error::AppError;
use kinetix_pricing_service::models::DiscountType;
use kinetix_pricing_service::services::{validate_percentage, PromotionService};
use kinetix_pricing_service::traits::applies_to_cart;

const SELLER: &str = "principal-of-the-seller";
const MERCHANT: &str = "merchant-principal-identity-names";

struct Merchants(Result<Option<MerchantStanding>, PeerUnavailable>);
struct Products(Result<HashMap<String, String>, PeerUnavailable>);

#[async_trait]
impl MerchantDirectoryPort for Merchants {
    async fn standing_of(
        &self,
        _principal_id: &str,
    ) -> Result<Option<MerchantStanding>, PeerUnavailable> {
        return self.0.clone();
    }
}

#[async_trait]
impl ProductDirectoryPort for Products {
    async fn owner_of(&self, sku: &str) -> Result<Option<String>, PeerUnavailable> {
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

fn unreachable(service: &str) -> PeerUnavailable {
    return PeerUnavailable(format!("{service} did not answer"));
}

fn promotions(merchants: Merchants, products: Products) -> PromotionService {
    return PromotionService::new(Arc::new(merchants), Arc::new(products));
}

#[tokio::test]
async fn an_admin_creates_a_platform_promotion_without_asking_anyone() {
    let service = promotions(
        Merchants(Err(unreachable("identity"))),
        Products(Err(unreachable("catalog"))),
    );

    let owner = service
        .owner_for("admin-principal", "admin", Some("SKU-1"))
        .await;

    assert!(matches!(owner, Ok(None)));
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
