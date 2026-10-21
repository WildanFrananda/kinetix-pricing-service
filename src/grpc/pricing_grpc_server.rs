use tonic::{Request, Response, Status};

use uuid::Uuid;

use rust_decimal::Decimal;

use crate::grpc::status_of::status_of;
use crate::models::{
    CalculatePriceRequest as DomainCalcReq, PriceItemRequest as DomainItemReq,
    ShippingQuoteRequest as DomainShippingQuote,
};
use crate::money::{from_money, from_optional_money, to_money, Money};
use crate::repositories::{QuotaOutcome, QuotaRepository};
use crate::services::PricingService;
use crate::DbPool;

use crate::proto::common::v1::ErrorDetail;
use crate::proto::pricing::v1 as proto;

use proto::pricing_service_server::PricingService as PricingGrpcTrait;
pub use proto::pricing_service_server::PricingServiceServer;
use proto::{
    AllocateFlashSaleStockRequest, AllocateFlashSaleStockResponse, CalculatePriceRequest,
    CalculatePriceResponse, PriceItemResponse, QuoteShippingRequest, QuoteShippingResponse,
    QuotedShipping, RedeemVoucherRequest, RedeemVoucherResponse, ReleaseFlashSaleAllocationRequest,
    ReleaseFlashSaleAllocationResponse, ReleaseVoucherRedemptionRequest,
    ReleaseVoucherRedemptionResponse,
};

pub struct PricingGrpcServer {
    pub pool: DbPool,
    pub pricing_service: PricingService<
        crate::repositories::DiscountRepository,
        crate::repositories::VoucherRepository,
        crate::repositories::FlashSaleRepository,
        crate::repositories::ShippingRateRepository,
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

            if base_price < Decimal::ZERO {
                return Err(Status::invalid_argument(format!(
                    "item {} has a negative base_price",
                    item.product_id
                )));
            }
            if item.quantity < 1 {
                return Err(Status::invalid_argument(format!(
                    "item {} has quantity {}; a priced line needs at least one unit",
                    item.product_id, item.quantity
                )));
            }

            domain_items.push(DomainItemReq {
                product_id: item.product_id,
                category_id: item.category_id,
                base_price,
                quantity: item.quantity,
            });
        }

        #[allow(deprecated)]
        let deprecated_caller_fee = from_optional_money(&req.base_shipping_fee)?;

        let gross = domain_items.iter().try_fold(
            deprecated_caller_fee.unwrap_or(Decimal::ZERO),
            |total, item| {
                return item
                    .base_price
                    .checked_mul(Decimal::from(item.quantity))
                    .and_then(|line| return total.checked_add(line));
            },
        );
        if !gross.is_some_and(|total| return to_money(total).is_some()) {
            return Err(Status::invalid_argument(
                "the lines of this cart add up to more than an amount of money can carry",
            ));
        }

        let domain_req = DomainCalcReq {
            items: domain_items,
            voucher_code: req.voucher_code,
            base_shipping_fee: deprecated_caller_fee,
            merchant_principal_id: req
                .merchant_principal_id
                .filter(|merchant| return !merchant.trim().is_empty()),
            shipping: req
                .shipping
                .map(|quote| {
                    return distance_of(&quote.service_tier, quote.distance_km).map(
                        |distance_km| {
                            return DomainShippingQuote {
                                service_tier: quote.service_tier,
                                distance_km,
                                total_weight_grams: quote.total_weight_grams,
                            };
                        },
                    );
                })
                .transpose()?,
        };

        let result = self
            .pricing_service
            .calculate_price(&self.pool, domain_req)
            .await
            .map_err(|e| return status_of(e, "CalculatePrice"))?;

        let mut pb_items = Vec::with_capacity(result.items.len());
        for item in result.items {
            pb_items.push(PriceItemResponse {
                base_price: Some(requested(item.base_price, "base_price")?),
                final_unit_price: Some(computed(item.final_unit_price, "final_unit_price")?),
                line_total: Some(computed(item.line_total, "line_total")?),
                product_id: item.product_id,
                quantity: item.quantity,
                applied_flash_sale: item.applied_flash_sale,
                applied_discount: item.applied_discount,
            });
        }

        return Ok(Response::new(CalculatePriceResponse {
            subtotal: Some(computed(result.subtotal, "subtotal")?),
            total_discount: Some(computed(result.total_discount, "total_discount")?),
            voucher_discount: Some(computed(result.voucher_discount, "voucher_discount")?),
            final_total: Some(computed(result.final_total, "final_total")?),
            applied_voucher: result.applied_voucher,
            items: pb_items,
            base_shipping_fee: Some(computed(result.base_shipping_fee, "base_shipping_fee")?),
            shipping_discount: Some(computed(Decimal::ZERO, "shipping_discount")?),
            final_shipping_fee: Some(computed(result.final_shipping_fee, "final_shipping_fee")?),
            payment_discount: Some(computed(Decimal::ZERO, "payment_discount")?),
        }));
    }

    async fn quote_shipping(
        &self,
        request: Request<QuoteShippingRequest>,
    ) -> Result<Response<QuoteShippingResponse>, Status> {
        let req = request.into_inner();

        let journeys = req
            .journeys
            .into_iter()
            .map(|journey| {
                return distance_of(&journey.service_tier, journey.distance_km).map(
                    |distance_km| {
                        return DomainShippingQuote {
                            service_tier: journey.service_tier,
                            distance_km,
                            total_weight_grams: journey.total_weight_grams,
                        };
                    },
                );
            })
            .collect::<Result<Vec<_>, Status>>()?;

        let quotes = self
            .pricing_service
            .quote_shipping(&self.pool, journeys)
            .await
            .map_err(|e| return status_of(e, "QuoteShipping"))?;

        let mut quoted = Vec::with_capacity(quotes.len());
        for quote in quotes {
            quoted.push(QuotedShipping {
                base_shipping_fee: Some(computed(quote.base_shipping_fee, "base_shipping_fee")?),
                service_tier: quote.service_tier,
                priced: quote.priced,
            });
        }

        return Ok(Response::new(QuoteShippingResponse { quotes: quoted }));
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
            req.merchant_principal_id
                .as_deref()
                .filter(|merchant| return !merchant.trim().is_empty()),
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
            QuotaOutcome::Expired => RedeemVoucherResponse {
                success: false,
                already_redeemed: false,
                remaining_quota: 0,
                error: Some(error_detail("VOUCHER_EXPIRED", "this voucher has expired")),
            },
            QuotaOutcome::NotFound => RedeemVoucherResponse {
                success: false,
                already_redeemed: false,
                remaining_quota: 0,
                error: Some(error_detail("VOUCHER_NOT_FOUND", "no such voucher")),
            },
            QuotaOutcome::NotForThisMerchant => RedeemVoucherResponse {
                success: false,
                already_redeemed: false,
                remaining_quota: 0,
                error: Some(error_detail(
                    "VOUCHER_NOT_FOR_THIS_MERCHANT",
                    "this voucher belongs to another merchant's store",
                )),
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
                .map_err(|e| Status::internal(format!("could not release the redemption: {e}")))?;

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
            QuotaOutcome::Exhausted
            | QuotaOutcome::Expired
            | QuotaOutcome::NotFound
            | QuotaOutcome::NotForThisMerchant => ReleaseVoucherRedemptionResponse {
                success: false,
                already_released: false,
                remaining_quota: 0,
                error: Some(error_detail("VOUCHER_NOT_FOUND", "no such voucher")),
            },
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
            QuotaOutcome::Expired | QuotaOutcome::NotFound | QuotaOutcome::NotForThisMerchant => {
                AllocateFlashSaleStockResponse {
                    success: false,
                    already_allocated: false,
                    remaining_stock: 0,
                    error: Some(error_detail("FLASH_SALE_NOT_FOUND", "no such flash sale")),
                }
            }
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

        let outcome = QuotaRepository::release_flash_sale(&self.pool, sale_id, &req.order_number)
            .await
            .map_err(|e| Status::internal(format!("could not release the allocation: {e}")))?;

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
            QuotaOutcome::Exhausted
            | QuotaOutcome::Expired
            | QuotaOutcome::NotFound
            | QuotaOutcome::NotForThisMerchant => ReleaseFlashSaleAllocationResponse {
                success: false,
                already_released: false,
                remaining_stock: 0,
                error: Some(error_detail("FLASH_SALE_NOT_FOUND", "no such flash sale")),
            },
        }));
    }
}

fn distance_of(service_tier: &str, distance_km: f64) -> Result<Decimal, Status> {
    return Decimal::try_from(distance_km)
        .ok()
        .filter(|km| return *km >= Decimal::ZERO)
        .ok_or_else(|| {
            return Status::invalid_argument(format!(
                "the {service_tier} journey has no usable distance: {distance_km} km"
            ));
        });
}

fn requested(amount: Decimal, field: &str) -> Result<Money, Status> {
    return to_money(amount).ok_or_else(|| {
        return Status::invalid_argument(format!(
            "{field} {amount} is more than an amount of money can carry"
        ));
    });
}

fn computed(amount: Decimal, field: &str) -> Result<Money, Status> {
    return to_money(amount).ok_or_else(|| {
        tracing::error!(field, amount = %amount, "a computed amount does not fit in Money");
        return Status::internal(format!(
            "pricing computed a {field} it cannot express as Money"
        ));
    });
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
