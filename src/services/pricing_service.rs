use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use crate::error::AppError;
use crate::models::{
    CalculatePriceRequest, CalculatePriceResponse, PriceItemResponse,
};
use crate::repositories::{
    DiscountRepository, DiscountRepositoryPort, FlashSaleRepository, FlashSaleRepositoryPort,
    VoucherRepository, VoucherRepositoryPort,
};
use crate::traits::{
    DefaultDiscountEvaluator, DefaultVoucherEvaluator, DiscountEvaluator, VoucherEvaluator,
};
use crate::DbPool;

pub struct PricingService<D, V, F>
where
    D: DiscountRepositoryPort,
    V: VoucherRepositoryPort,
    F: FlashSaleRepositoryPort,
{
    pub discount_repo: D,
    pub voucher_repo: V,
    pub flash_sale_repo: F,
    pub discount_evaluator: Box<dyn DiscountEvaluator>,
    pub voucher_evaluator: Box<dyn VoucherEvaluator>,
}

impl Default for PricingService<DiscountRepository, VoucherRepository, FlashSaleRepository> {
    fn default() -> Self {
        return Self::new(
            DiscountRepository,
            VoucherRepository,
            FlashSaleRepository,
            Box::new(DefaultDiscountEvaluator),
            Box::new(DefaultVoucherEvaluator),
        );
    }
}

impl<D, V, F> PricingService<D, V, F>
where
    D: DiscountRepositoryPort,
    V: VoucherRepositoryPort,
    F: FlashSaleRepositoryPort,
{
    pub fn new(
        discount_repo: D,
        voucher_repo: V,
        flash_sale_repo: F,
        discount_evaluator: Box<dyn DiscountEvaluator>,
        voucher_evaluator: Box<dyn VoucherEvaluator>,
    ) -> Self {
        return Self {
            discount_repo,
            voucher_repo,
            flash_sale_repo,
            discount_evaluator,
            voucher_evaluator,
        };
    }

    pub async fn calculate_price(
        &self,
        pool: &DbPool,
        req: CalculatePriceRequest,
    ) -> Result<CalculatePriceResponse, AppError> {
        let active_discounts = self.discount_repo.find_all_active(pool).await?;
        let mut item_responses = Vec::new();
        let mut subtotal = dec!(0.00);
        let mut total_item_savings = dec!(0.00);

        for item in &req.items {
            let base_price = item.base_price;
            let mut final_unit_price = base_price;
            let mut applied_flash_sale = None;
            let mut applied_discount = None;

            if let Ok(Some(flash)) = self.flash_sale_repo.find_active_for_product(pool, &item.product_id).await {
                if flash.flash_price < final_unit_price {
                    final_unit_price = flash.flash_price;
                    applied_flash_sale = Some(flash.title.clone());
                }
            }

            if applied_flash_sale.is_none() {
                let mut best_discount_savings = dec!(0.00);
                let mut selected_discount_title = None;

                for discount in &active_discounts {
                    if self.discount_evaluator.matches_item(discount, item) {
                        let savings = self.discount_evaluator.calculate_savings(discount, base_price);
                        if savings > best_discount_savings {
                            best_discount_savings = savings;
                            selected_discount_title = Some(discount.title.clone());
                        }
                    }
                }

                if best_discount_savings > dec!(0.00) {
                    final_unit_price = (base_price - best_discount_savings).max(dec!(0.00));
                    applied_discount = selected_discount_title;
                }
            }

            let quantity_dec = Decimal::from(item.quantity);
            let line_total = final_unit_price * quantity_dec;
            let base_line_total = base_price * quantity_dec;

            subtotal += line_total;
            total_item_savings += base_line_total - line_total;

            item_responses.push(PriceItemResponse {
                product_id: item.product_id.clone(),
                base_price,
                final_unit_price,
                quantity: item.quantity,
                line_total,
                applied_flash_sale,
                applied_discount,
            });
        }

        let mut voucher_discount = dec!(0.00);
        let mut shipping_discount = dec!(0.00);
        let mut applied_voucher = None;
        let base_shipping = req.base_shipping_fee.unwrap_or(dec!(0.00));

        if let Some(code) = &req.voucher_code {
            if let Ok(Some(voucher)) = self.voucher_repo.find_by_code(pool, code).await {
                if self.voucher_evaluator.is_eligible(&voucher, subtotal) {
                    if voucher.discount_type == "SHIPPING" || voucher.code.contains("FREE_SHIP") {
                        shipping_discount = base_shipping.min(self.voucher_evaluator.calculate_discount(&voucher, base_shipping));
                    } else {
                        voucher_discount = self.voucher_evaluator.calculate_discount(&voucher, subtotal);
                    }
                    applied_voucher = Some(voucher.code.clone());
                }
            }
        }

        let final_shipping_fee = (base_shipping - shipping_discount).max(dec!(0.00));
        let final_total = (subtotal - voucher_discount + final_shipping_fee).max(dec!(0.00));

        return Ok(CalculatePriceResponse {
            subtotal,
            total_discount: total_item_savings + voucher_discount + shipping_discount,
            voucher_discount,
            final_total,
            applied_voucher,
            items: item_responses,
            base_shipping_fee: base_shipping,
            shipping_discount,
            final_shipping_fee,
        });
    }
}
