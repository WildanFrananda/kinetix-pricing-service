#[cfg(test)]
mod tests {
    use rust_decimal_macros::dec;
    use kinetix_pricing_service::models::discount::DiscountType;
    use kinetix_pricing_service::models::voucher::ApplyVoucherRequest;
    use kinetix_pricing_service::error::AppError;

    #[test]
    fn test_discount_type_conversions() {
        assert_eq!(DiscountType::Percentage.as_str(), "PERCENTAGE");
        assert_eq!(DiscountType::Fixed.as_str(), "FIXED");

        let parsed_fixed: DiscountType = "FIXED".parse().unwrap();
        assert_eq!(parsed_fixed, DiscountType::Fixed);

        let parsed_pct: DiscountType = "percentage".parse().unwrap();
        assert_eq!(parsed_pct, DiscountType::Percentage);

        assert!("INVALID".parse::<DiscountType>().is_err());
    }

    #[test]
    fn test_apply_voucher_request() {
        let req = ApplyVoucherRequest::new("PROMO50".to_string(), dec!(150000.0));
        assert_eq!(req.code, "PROMO50");
        assert_eq!(req.cart_subtotal, dec!(150000.0));
    }

    #[test]
    fn test_app_error_display() {
        let err_not_found = AppError::NotFound("Item not found".to_string());
        assert_eq!(err_not_found.to_string(), "Not found: Item not found");

        let err_bad_req = AppError::BadRequest("Invalid payload".to_string());
        assert_eq!(err_bad_req.to_string(), "Validation error: Invalid payload");

        let err_unauth = AppError::Unauthorized;
        assert_eq!(err_unauth.to_string(), "Unauthorized access");
    }
}
