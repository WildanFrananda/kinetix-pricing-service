use std::sync::Arc;

use crate::clients::{MerchantDirectoryPort, MerchantStanding, PeerError, ProductDirectoryPort};
use crate::error::AppError;
use crate::guards::AuthenticatedCaller;
use crate::models::{PublicVoucher, Voucher, VoucherView};

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
            if let Some(sku) = product_sku {
                self.product_owner(sku).await?;
            }
            return Ok(None);
        }

        if role != "seller" {
            return Err(AppError::Forbidden(
                "only an admin or a seller may create a promotion".to_string(),
            ));
        }

        let standing = self.standing_of(principal_id).await?.ok_or_else(|| {
            return AppError::Forbidden("identity knows no merchant for this account".to_string());
        })?;

        if !standing.may_sell {
            return Err(AppError::Forbidden(
                "identity does not permit this merchant to trade".to_string(),
            ));
        }

        if let Some(sku) = product_sku {
            let owner = self.product_owner(sku).await?;

            if owner != standing.merchant_principal_id {
                return Err(AppError::Forbidden(format!(
                    "product {sku} belongs to another merchant"
                )));
            }
        }

        return Ok(Some(standing.merchant_principal_id));
    }

    pub async fn voucher_view(
        &self,
        voucher: Voucher,
        viewer: Option<&AuthenticatedCaller>,
    ) -> Result<VoucherView, AppError> {
        if self.sees_terms(&voucher, viewer).await? {
            return Ok(VoucherView::Full(voucher));
        }

        return Ok(VoucherView::Public(PublicVoucher::from(voucher)));
    }

    async fn sees_terms(
        &self,
        voucher: &Voucher,
        viewer: Option<&AuthenticatedCaller>,
    ) -> Result<bool, AppError> {
        let Some(viewer) = viewer else {
            return Ok(false);
        };

        if viewer.role == "admin" {
            return Ok(true);
        }

        let Some(owner) = voucher.merchant_principal_id.as_deref() else {
            return Ok(false);
        };

        if viewer.role != "seller" {
            return Ok(false);
        }

        let standing = self.standing_of(&viewer.principal_id).await?;

        return Ok(standing.is_some_and(|standing| {
            return standing.merchant_principal_id == owner;
        }));
    }

    async fn standing_of(&self, principal_id: &str) -> Result<Option<MerchantStanding>, AppError> {
        match self.merchants.standing_of(principal_id).await {
            Ok(standing) => return Ok(standing),
            Err(PeerError::Refused(reason)) => {
                tracing::warn!(error = %reason, "identity refused a merchant lookup");
                return Ok(None);
            }
            Err(PeerError::Unavailable(reason)) => return Err(AppError::Unavailable(reason)),
        }
    }

    async fn product_owner(&self, sku: &str) -> Result<String, AppError> {
        match self.products.owner_of(sku).await {
            Ok(Some(owner)) => return Ok(owner),
            Ok(None) => return Err(AppError::NotFound(format!("catalog has no product {sku}"))),
            Err(PeerError::Refused(reason)) => {
                return Err(AppError::BadRequest(format!(
                    "catalog would not look up product '{sku}': {reason}"
                )));
            }
            Err(PeerError::Unavailable(reason)) => return Err(AppError::Unavailable(reason)),
        }
    }
}
