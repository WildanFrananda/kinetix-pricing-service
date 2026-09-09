use prost::Message;
use prost_types::FileDescriptorSet;

use kinetix_pricing_service::observability::grpc_labels::{
    grpc_method_label, KNOWN_GRPC_METHODS, UNKNOWN_GRPC_METHOD,
};

#[test]
fn every_rpc_in_the_wire_contract_has_a_label() {
    let descriptor =
        FileDescriptorSet::decode(&tonic::include_file_descriptor_set!("pricing_descriptor")[..])
            .expect("the descriptor set build.rs wrote is not a FileDescriptorSet");

    for file in &descriptor.file {
        for service in &file.service {
            for method in &service.method {
                let path = format!("{}.{}/{}", file.package(), service.name(), method.name());
                assert!(
                    KNOWN_GRPC_METHODS.contains(&path.as_str()),
                    "{path} is served but has no metric label; add it to KNOWN_GRPC_METHODS"
                );
            }
        }
    }
}

#[test]
fn a_known_method_keeps_its_name_with_or_without_the_leading_slash() {
    assert_eq!(
        grpc_method_label("/pricing.v1.PricingService/CalculatePrice"),
        "pricing.v1.PricingService/CalculatePrice"
    );
    assert_eq!(
        grpc_method_label("pricing.v1.PricingService/CalculatePrice"),
        "pricing.v1.PricingService/CalculatePrice"
    );
}

#[test]
fn a_path_nobody_serves_is_bounded_rather_than_echoed() {
    assert_eq!(
        grpc_method_label("/pricing.v1.PricingService/8f3a1c22-0e5b-4a91-9f0d-2b7c11d4e6aa"),
        UNKNOWN_GRPC_METHOD
    );
}
