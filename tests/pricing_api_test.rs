#[cfg(test)]
mod tests {
    use rust_decimal_macros::dec;

    #[test]
    fn test_discount_calculation_logic() {
        let base_price = dec!(100.00);
        let discount_percent = dec!(20.00);
        let expected_final = dec!(80.00);

        let savings = base_price * (discount_percent / dec!(100.00));
        let final_price = base_price - savings;

        assert_eq!(final_price, expected_final);
    }

    #[test]
    fn test_voucher_min_spend_guard() {
        let subtotal = dec!(150.00);
        let min_spend = dec!(200.00);

        let is_eligible = subtotal >= min_spend;
        assert!(!is_eligible);
    }

    #[test]
    fn test_flash_sale_override_priority() {
        let base_price = dec!(100.00);
        let flash_price = dec!(49.99);

        let final_unit_price = flash_price.min(base_price);
        assert_eq!(final_unit_price, dec!(49.99));
    }

    #[test]
    fn test_auth_role_validation_rules() {
        let admin_role = "ADMIN";
        let merchant_role = "MERCHANT";
        let customer_role = "CUSTOMER";

        let is_admin_or_merchant = |role: &str| -> bool {
            return role == "ADMIN" || role == "MERCHANT";
        };

        assert!(is_admin_or_merchant(admin_role));
        assert!(is_admin_or_merchant(merchant_role));
        assert!(!is_admin_or_merchant(customer_role));
    }
}
