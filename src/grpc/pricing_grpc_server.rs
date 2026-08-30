use rust_decimal::Decimal;
use std::str::FromStr;
use tonic::{Request, Response, Status};

use crate::models::{CalculatePriceRequest as DomainCalcReq, PriceItemRequest as DomainItemReq};
use crate::services::PricingService;
use crate::DbPool;

pub mod proto {
    tonic::include_proto!("pricing.v1");
}

use proto::pricing_service_server::PricingService as PricingGrpcTrait;
pub use proto::pricing_service_server::PricingServiceServer;
use proto::{CalculatePriceRequest, CalculatePriceResponse, PriceItemResponse};

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
            let base_price = Decimal::from_str(&item.base_price).map_err(|_| {
                return Status::invalid_argument(format!("Invalid base_price decimal: {}", item.base_price));
            })?;

            domain_items.push(DomainItemReq {
                product_id: item.product_id,
                category_id: item.category_id,
                base_price,
                quantity: item.quantity,
            });
        }

        let base_shipping_fee = match req.base_shipping_fee {
            Some(fee_str) => Decimal::from_str(&fee_str).ok(),
            None => None,
        };

        let domain_req = DomainCalcReq {
            items: domain_items,
            voucher_code: req.voucher_code,
            base_shipping_fee,
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
                    base_price: item.base_price.to_string(),
                    final_unit_price: item.final_unit_price.to_string(),
                    quantity: item.quantity,
                    line_total: item.line_total.to_string(),
                    applied_flash_sale: item.applied_flash_sale,
                    applied_discount: item.applied_discount,
                };
            })
            .collect();

        return Ok(Response::new(CalculatePriceResponse {
            subtotal: result.subtotal.to_string(),
            total_discount: result.total_discount.to_string(),
            voucher_discount: result.voucher_discount.to_string(),
            final_total: result.final_total.to_string(),
            applied_voucher: result.applied_voucher,
            items: pb_items,
            base_shipping_fee: result.base_shipping_fee.to_string(),
            shipping_discount: result.shipping_discount.to_string(),
            final_shipping_fee: result.final_shipping_fee.to_string(),
            payment_discount: result.payment_discount.to_string(),
        }));
    }
}
