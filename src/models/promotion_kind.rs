#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromotionKind {
    Discount,
    Voucher,
    FlashSale,
}

impl PromotionKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            PromotionKind::Discount => return "DISCOUNT",
            PromotionKind::Voucher => return "VOUCHER",
            PromotionKind::FlashSale => return "FLASH_SALE",
        }
    }
}
