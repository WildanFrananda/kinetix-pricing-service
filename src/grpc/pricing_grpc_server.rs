use tonic::{Request, Response, Status};

use uuid::Uuid;

use crate::models::{CalculatePriceRequest as DomainCalcReq, PriceItemRequest as DomainItemReq};
use crate::money::{from_money, from_optional_money, to_money};
use crate::repositories::{QuotaOutcome, QuotaRepository};
use crate::services::PricingService;
use crate::DbPool;

use crate::proto::common::v1::ErrorDetail;
use crate::proto::pricing::v1 as proto;

use proto::pricing_service_server::PricingService as PricingGrpcTrait;
pub use proto::pricing_service_server::PricingServiceServer;
use proto::{
    AllocateFlashSaleStockRequest, AllocateFlashSaleStockResponse, CalculatePriceRequest,
    CalculatePriceResponse, PriceItemResponse, RedeemVoucherRequest, RedeemVoucherResponse,
    ReleaseFlashSaleAllocationRequest, ReleaseFlashSaleAllocationResponse,
    ReleaseVoucherRedemptionRequest, ReleaseVoucherRedemptionResponse,
};

pub struct PricingGrpcServer {
    pub pool: DbPool,
    pub pricing_service: PricingService<
        crate::repositories::DiscountRepository,
        crate::repositories::VoucherRepository,
        crate::repositories::FlashSaleRepository,
    >,
}

impl PricingGrpcServer {
    pub fn new(pool: DbPool) -> Self {
        return PricingGrpcServer {
            pool,
            pricing_service: PricingService::default(),
        };
    }
}

#[tonic::async_trait]
impl PricingGrpcTrait for PricingGrpcServer {
    async fn calculate_price(
        &self,
        request: Request<CalculatePriceRequest>,
    ) -> Result<Response<CalculatePriceResponse>, Status> {
        let req = request.into_inner();
        let mut domain_items = Vec::new();

        for item in req.items {
            let base_price = match &item.base_price {
                Some(money) => from_money(money)?,
                None => {
                    return Err(Status::invalid_argument(format!(
                        "item {} carries no base_price",
                        item.product_id
                    )))
                }
            };

            domain_items.push(DomainItemReq {
                product_id: item.product_id,
                category_id: item.category_id,
                base_price,
                quantity: item.quantity,
            });
        }

        let domain_req = DomainCalcReq {
            items: domain_items,
            voucher_code: req.voucher_code,
            base_shipping_fee: from_optional_money(&req.base_shipping_fee)?,
            payment_method: req.payment_method,
        };

        let result = self
            .pricing_service
            .calculate_price(&self.pool, domain_req)
            .await
            .map_err(|e| {
                return Status::internal(format!("Pricing service calculation error: {}", e));
            })?;

        let pb_items = result
            .items
            .into_iter()
            .map(|item| {
                return PriceItemResponse {
                    product_id: item.product_id,
                    base_price: Some(to_money(item.base_price)),
                    final_unit_price: Some(to_money(item.final_unit_price)),
                    quantity: item.quantity,
                    line_total: Some(to_money(item.line_total)),
                    applied_flash_sale: item.applied_flash_sale,
                    applied_discount: item.applied_discount,
                };
            })
            .collect();

        return Ok(Response::new(CalculatePriceResponse {
            subtotal: Some(to_money(result.subtotal)),
            total_discount: Some(to_money(result.total_discount)),
            voucher_discount: Some(to_money(result.voucher_discount)),
            final_total: Some(to_money(result.final_total)),
            applied_voucher: result.applied_voucher,
            items: pb_items,
            base_shipping_fee: Some(to_money(result.base_shipping_fee)),
            shipping_discount: Some(to_money(result.shipping_discount)),
            final_shipping_fee: Some(to_money(result.final_shipping_fee)),
            payment_discount: Some(to_money(result.payment_discount)),
        }));
    }

    async fn redeem_voucher(
        &self,
        request: Request<RedeemVoucherRequest>,
    ) -> Result<Response<RedeemVoucherResponse>, Status> {
        let req = request.into_inner();

        if req.voucher_code.trim().is_empty() || req.order_number.trim().is_empty() {
            return Ok(Response::new(RedeemVoucherResponse {
                success: false,
                already_redeemed: false,
                remaining_quota: 0,
                error: Some(invalid_argument(
                    "a voucher code and an order number are both required",
                )),
            }));
        }

        let outcome = QuotaRepository::redeem_voucher(
            &self.pool,
            &req.voucher_code,
            &req.order_number,
            &req.customer_principal_id,
        )
        .await
        .map_err(|e| Status::internal(format!("could not redeem the voucher: {e}")))?;

        return Ok(Response::new(match outcome {
            QuotaOutcome::Applied { remaining } => RedeemVoucherResponse {
                success: true,
                already_redeemed: false,
                remaining_quota: remaining,
                error: None,
            },
            QuotaOutcome::AlreadyDone { remaining } => RedeemVoucherResponse {
                success: true,
                already_redeemed: true,
                remaining_quota: remaining,
                error: None,
            },
            QuotaOutcome::Exhausted => RedeemVoucherResponse {
                success: false,
                already_redeemed: false,
                remaining_quota: 0,
                error: Some(error_detail(
                    "VOUCHER_QUOTA_EXHAUSTED",
                    "this voucher has no quota left",
                )),
            },
            QuotaOutcome::NotFound => RedeemVoucherResponse {
                success: false,
                already_redeemed: false,
                remaining_quota: 0,
                error: Some(error_detail("VOUCHER_NOT_FOUND", "no such voucher")),
            },
        }));
    }

    async fn release_voucher_redemption(
        &self,
        request: Request<ReleaseVoucherRedemptionRequest>,
    ) -> Result<Response<ReleaseVoucherRedemptionResponse>, Status> {
        let req = request.into_inner();

        if req.voucher_code.trim().is_empty() || req.order_number.trim().is_empty() {
            return Ok(Response::new(ReleaseVoucherRedemptionResponse {
                success: false,
                already_released: false,
                remaining_quota: 0,
                error: Some(invalid_argument(
                    "a voucher code and an order number are both required",
                )),
            }));
        }

        let outcome =
            QuotaRepository::release_voucher(&self.pool, &req.voucher_code, &req.order_number)
                .await
                .map_err(|e| {
                    Status::internal(format!("could not release the redemption: {e}"))
                })?;

        return Ok(Response::new(match outcome {
            QuotaOutcome::Applied { remaining } => ReleaseVoucherRedemptionResponse {
                success: true,
                already_released: false,
                remaining_quota: remaining,
                error: None,
            },
            QuotaOutcome::AlreadyDone { remaining } => ReleaseVoucherRedemptionResponse {
                success: true,
                already_released: true,
                remaining_quota: remaining,
                error: None,
            },
            QuotaOutcome::Exhausted | QuotaOutcome::NotFound => {
                ReleaseVoucherRedemptionResponse {
                    success: false,
                    already_released: false,
                    remaining_quota: 0,
                    error: Some(error_detail("VOUCHER_NOT_FOUND", "no such voucher")),
                }
            }
        }));
    }

    async fn allocate_flash_sale_stock(
        &self,
        request: Request<AllocateFlashSaleStockRequest>,
    ) -> Result<Response<AllocateFlashSaleStockResponse>, Status> {
        let req = request.into_inner();

        let Ok(sale_id) = Uuid::parse_str(&req.flash_sale_id) else {
            return Ok(Response::new(AllocateFlashSaleStockResponse {
                success: false,
                already_allocated: false,
                remaining_stock: 0,
                error: Some(invalid_argument("flash_sale_id is not a UUID")),
            }));
        };

        let outcome = QuotaRepository::allocate_flash_sale(
            &self.pool,
            sale_id,
            &req.product_id,
            req.quantity,
            &req.order_number,
        )
        .await
        .map_err(|e| Status::internal(format!("could not allocate flash-sale stock: {e}")))?;

        return Ok(Response::new(match outcome {
            QuotaOutcome::Applied { remaining } => AllocateFlashSaleStockResponse {
                success: true,
                already_allocated: false,
                remaining_stock: remaining,
                error: None,
            },
            QuotaOutcome::AlreadyDone { remaining } => AllocateFlashSaleStockResponse {
                success: true,
                already_allocated: true,
                remaining_stock: remaining,
                error: None,
            },
            QuotaOutcome::Exhausted => AllocateFlashSaleStockResponse {
                success: false,
                already_allocated: false,
                remaining_stock: 0,
                error: Some(error_detail(
                    "FLASH_SALE_STOCK_EXHAUSTED",
                    "this flash sale has not got that many units left",
                )),
            },
            QuotaOutcome::NotFound => AllocateFlashSaleStockResponse {
                success: false,
                already_allocated: false,
                remaining_stock: 0,
                error: Some(error_detail("FLASH_SALE_NOT_FOUND", "no such flash sale")),
            },
        }));
    }

    async fn release_flash_sale_allocation(
        &self,
        request: Request<ReleaseFlashSaleAllocationRequest>,
    ) -> Result<Response<ReleaseFlashSaleAllocationResponse>, Status> {
        let req = request.into_inner();

        let Ok(sale_id) = Uuid::parse_str(&req.flash_sale_id) else {
            return Ok(Response::new(ReleaseFlashSaleAllocationResponse {
                success: false,
                already_released: false,
                remaining_stock: 0,
                error: Some(invalid_argument("flash_sale_id is not a UUID")),
            }));
        };

        let outcome =
            QuotaRepository::release_flash_sale(&self.pool, sale_id, &req.order_number)
                .await
                .map_err(|e| {
                    Status::internal(format!("could not release the allocation: {e}"))
                })?;

        return Ok(Response::new(match outcome {
            QuotaOutcome::Applied { remaining } => ReleaseFlashSaleAllocationResponse {
                success: true,
                already_released: false,
                remaining_stock: remaining,
                error: None,
            },
            QuotaOutcome::AlreadyDone { remaining } => ReleaseFlashSaleAllocationResponse {
                success: true,
                already_released: true,
                remaining_stock: remaining,
                error: None,
            },
            QuotaOutcome::Exhausted | QuotaOutcome::NotFound => {
                ReleaseFlashSaleAllocationResponse {
                    success: false,
                    already_released: false,
                    remaining_stock: 0,
                    error: Some(error_detail("FLASH_SALE_NOT_FOUND", "no such flash sale")),
                }
            }
        }));
    }
}

fn error_detail(code: &str, message: &str) -> ErrorDetail {
    return ErrorDetail {
        error_code: code.to_string(),
        message: message.to_string(),
        field_violations: Vec::new(),
    };
}

fn invalid_argument(message: &str) -> ErrorDetail {
    return error_detail("INVALID_ARGUMENT", message);
}
