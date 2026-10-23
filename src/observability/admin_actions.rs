pub const ADMIN_ACTION_EVENT: &str = "admin_action";

pub const DISCOUNT_CREATE: &str = "pricing.discount_create";
pub const VOUCHER_CREATE: &str = "pricing.voucher_create";
pub const FLASH_SALE_CREATE: &str = "pricing.flash_sale_create";
pub const VOUCHER_READ: &str = "pricing.voucher_read";
pub const VOUCHER_APPLY: &str = "pricing.voucher_apply";

pub const KNOWN_ADMIN_ACTIONS: &[&str] = &[
    DISCOUNT_CREATE,
    VOUCHER_CREATE,
    FLASH_SALE_CREATE,
    VOUCHER_READ,
    VOUCHER_APPLY,
];

pub const NO_TARGET: &str = "-";

const AUDITED_ROUTES: &[(&str, &str)] = &[
    ("create_discount", DISCOUNT_CREATE),
    ("create_voucher", VOUCHER_CREATE),
    ("create_flash_sale", FLASH_SALE_CREATE),
    ("get_voucher", VOUCHER_READ),
    ("apply_voucher", VOUCHER_APPLY),
];

pub fn admin_action_of_route(route_name: &str) -> Option<&'static str> {
    return AUDITED_ROUTES
        .iter()
        .find(|(name, _)| return *name == route_name)
        .map(|(_, action)| return *action);
}
