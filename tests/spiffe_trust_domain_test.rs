use kinetix_pricing_service::security::spiffe::{service_of, trust_domain};

#[test]
fn an_unset_variable_keeps_the_domain_the_estate_runs_today() {
    assert_eq!(trust_domain(), "kinetix.local");
}

#[test]
fn an_id_from_the_configured_domain_names_its_service() {
    assert_eq!(
        service_of("spiffe://kinetix.local/service/order", "kinetix.local"),
        Some("order".to_string())
    );
}

#[test]
fn a_different_domain_can_be_configured_without_touching_this_code() {
    assert_eq!(
        service_of("spiffe://prod.kinetix/service/order", "prod.kinetix"),
        Some("order".to_string())
    );
}

#[test]
fn an_id_from_another_trust_domain_names_nobody() {
    assert_eq!(service_of("spiffe://prod.kinetix/service/order", "kinetix.local"), None);
    assert_eq!(service_of("spiffe://kinetix.local/service/order", "prod.kinetix"), None);
}

#[test]
fn a_domain_this_one_is_merely_a_prefix_of_is_refused() {
    assert_eq!(
        service_of("spiffe://kinetix.local.example.com/service/order", "kinetix.local"),
        None
    );
}

#[test]
fn an_id_that_names_no_service_is_refused() {
    assert_eq!(service_of("spiffe://kinetix.local/order", "kinetix.local"), None);
    assert_eq!(service_of("https://kinetix.local/service/order", "kinetix.local"), None);
}
