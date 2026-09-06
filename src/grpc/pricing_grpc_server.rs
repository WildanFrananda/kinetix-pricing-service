use tonic::{Request, Response, Status};

use crate::models::{CalculatePriceRequest as DomainCalcReq, PriceItemRequest as DomainItemReq};
use crate::money::{from_money, from_optional_money, to_money};
use crate::services::PricingService;
use crate::DbPool;

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
        _request: Request<RedeemVoucherRequest>,
    ) -> Result<Response<RedeemVoucherResponse>, Status> {
        return Err(Status::unimplemented(
            "RedeemVoucher lands with the quota ledger in S10",
        ));
    }

    async fn release_voucher_redemption(
        &self,
        _request: Request<ReleaseVoucherRedemptionRequest>,
    ) -> Result<Response<ReleaseVoucherRedemptionResponse>, Status> {
        return Err(Status::unimplemented(
            "ReleaseVoucherRedemption lands with the quota ledger in S10",
        ));
    }

    async fn allocate_flash_sale_stock(
        &self,
        _request: Request<AllocateFlashSaleStockRequest>,
    ) -> Result<Response<AllocateFlashSaleStockResponse>, Status> {
        return Err(Status::unimplemented(
            "AllocateFlashSaleStock lands with the quota ledger in S10",
        ));
    }

    async fn release_flash_sale_allocation(
        &self,
        _request: Request<ReleaseFlashSaleAllocationRequest>,
    ) -> Result<Response<ReleaseFlashSaleAllocationResponse>, Status> {
        return Err(Status::unimplemented(
            "ReleaseFlashSaleAllocation lands with the quota ledger in S10",
        ));
    }
}
