use std::sync::Arc;

use rust_decimal_macros::dec;

use crate::clients::{MerchantDirectoryPort, ProductDirectoryPort};
use crate::error::AppError;
use crate::models::DiscountType;

pub struct PromotionService {
    merchants: Arc<dyn MerchantDirectoryPort>,
    products: Arc<dyn ProductDirectoryPort>,
}

impl PromotionService {
    pub fn new(
        merchants: Arc<dyn MerchantDirectoryPort>,
        products: Arc<dyn ProductDirectoryPort>,
    ) -> Self {
        return Self {
            merchants,
            products,
        };
    }

    pub async fn owner_for(
        &self,
        principal_id: &str,
        role: &str,
        product_sku: Option<&str>,
    ) -> Result<Option<String>, AppError> {
        if role == "admin" {
            return Ok(None);
        }

        if role != "seller" {
            return Err(AppError::Forbidden(
                "only an admin or a seller may create a promotion".to_string(),
            ));
        }

        let standing = self
            .merchants
            .standing_of(principal_id)
            .await
            .map_err(|unavailable| return AppError::Unavailable(unavailable.0))?
            .ok_or_else(|| {
                return AppError::Forbidden(
                    "identity knows no merchant for this account".to_string(),
                );
            })?;

        if !standing.may_sell {
            return Err(AppError::Forbidden(
                "identity does not permit this merchant to trade".to_string(),
            ));
        }

        if let Some(sku) = product_sku {
            let owner = self
                .products
                .owner_of(sku)
                .await
                .map_err(|unavailable| return AppError::Unavailable(unavailable.0))?
                .ok_or_else(|| {
                    return AppError::NotFound(format!("catalog has no product {sku}"));
                })?;

            if owner != standing.merchant_principal_id {
                return Err(AppError::Forbidden(format!(
                    "product {sku} belongs to another merchant"
                )));
            }
        }

        return Ok(Some(standing.merchant_principal_id));
    }
}

pub fn validate_percentage(
    kind: &DiscountType,
    value: rust_decimal::Decimal,
) -> Result<(), AppError> {
    if *kind == DiscountType::Percentage && value > dec!(100) {
        return Err(AppError::BadRequest(
            "a percentage cannot be more than 100".to_string(),
        ));
    }
    return Ok(());
}
