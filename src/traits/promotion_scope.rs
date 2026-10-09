pub fn applies_to_cart(owner: Option<&str>, cart_merchant: Option<&str>) -> bool {
    match owner {
        None => return true,
        Some(merchant) => return cart_merchant == Some(merchant),
    }
}
