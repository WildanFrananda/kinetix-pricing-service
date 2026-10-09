use tonic::{Code, Request};

use kinetix_pricing_service::db::create_pool;
use kinetix_pricing_service::grpc::PricingGrpcServer;
use kinetix_pricing_service::proto::pricing::v1::pricing_service_server::PricingService;
use kinetix_pricing_service::proto::pricing::v1::{
    CalculatePriceRequest, QuoteShippingRequest, ShippingQuoteRequest,
};

fn server() -> PricingGrpcServer {
    return PricingGrpcServer::new(create_pool("postgres://unused:unused@127.0.0.1:1/unused"));
}

fn journey(distance_km: f64) -> ShippingQuoteRequest {
    return ShippingQuoteRequest {
        service_tier: "KINETIX_INSTANT".to_string(),
        distance_km,
        total_weight_grams: 1_000,
    };
}

#[tokio::test]
async fn a_journey_with_no_usable_distance_is_refused_rather_than_priced_as_zero_km() {
    for distance in [f64::NAN, f64::INFINITY, -1.0] {
        let quote = server()
            .quote_shipping(Request::new(QuoteShippingRequest {
                journeys: vec![journey(distance)],
            }))
            .await;
        assert_eq!(
            quote.err().map(|status| return status.code()),
            Some(Code::InvalidArgument),
            "QuoteShipping at {distance} km"
        );

        let price = server()
            .calculate_price(Request::new(CalculatePriceRequest {
                shipping: Some(journey(distance)),
                ..Default::default()
            }))
            .await;
        assert_eq!(
            price.err().map(|status| return status.code()),
            Some(Code::InvalidArgument),
            "CalculatePrice at {distance} km"
        );
    }
}
