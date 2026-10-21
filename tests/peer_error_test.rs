use tonic::{Code, Status};

use kinetix_pricing_service::clients::PeerError;

#[test]
fn an_argument_the_peer_refused_is_a_refusal_with_its_reason() {
    let error = PeerError::from_status(
        "catalog",
        "GetProduct",
        &Status::invalid_argument("sku is required"),
    );

    assert_eq!(
        error,
        PeerError::Refused("catalog refused GetProduct: sku is required".to_string())
    );
}

#[test]
fn a_peer_that_failed_or_could_not_be_reached_is_unavailable() {
    for code in [
        Code::Unavailable,
        Code::DeadlineExceeded,
        Code::Internal,
        Code::Unknown,
        Code::ResourceExhausted,
        Code::PermissionDenied,
        Code::Unauthenticated,
        Code::Cancelled,
    ] {
        let error = PeerError::from_status("identity", "GetMerchantInfo", &Status::new(code, "x"));

        assert!(
            matches!(&error, PeerError::Unavailable(reason) if reason.contains("GetMerchantInfo")),
            "{code:?} became {error:?}"
        );
    }
}
