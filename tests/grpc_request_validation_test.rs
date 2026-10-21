use tonic::{Code, Request};

use kinetix_pricing_service::db::create_pool;
use kinetix_pricing_service::error::AppError;
use kinetix_pricing_service::grpc::{status_of, PricingGrpcServer};
use kinetix_pricing_service::money::Money;
use kinetix_pricing_service::proto::pricing::v1::pricing_service_server::PricingService;
use kinetix_pricing_service::proto::pricing::v1::{
    CalculatePriceRequest, PriceItemRequest, QuoteShippingRequest, ShippingQuoteRequest,
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

fn line(quantity: i32, amount_minor: i64) -> PriceItemRequest {
    return PriceItemRequest {
        product_id: "SKU-1".to_string(),
        base_price: Some(Money {
            amount_minor,
            currency: "IDR".to_string(),
        }),
        quantity,
        ..Default::default()
    };
}

#[tokio::test]
async fn a_line_with_no_units_is_refused_rather_than_priced_into_a_smaller_subtotal() {
    for quantity in [0, -1, i32::MIN] {
        let price = server()
            .calculate_price(Request::new(CalculatePriceRequest {
                items: vec![line(quantity, 1_000_000)],
                ..Default::default()
            }))
            .await;

        assert_eq!(
            price.err().map(|status| return status.code()),
            Some(Code::InvalidArgument),
            "quantity {quantity}"
        );
    }
}

#[tokio::test]
async fn a_line_with_a_negative_price_is_refused() {
    let price = server()
        .calculate_price(Request::new(CalculatePriceRequest {
            items: vec![line(1, -1)],
            ..Default::default()
        }))
        .await;

    assert_eq!(
        price.err().map(|status| return status.code()),
        Some(Code::InvalidArgument)
    );
}

#[test]
fn a_refusal_inside_pricing_reaches_the_peer_as_a_refusal_and_an_outage_as_unavailable() {
    let cases = [
        (
            AppError::BadRequest("no shipping rate".to_string()),
            Code::InvalidArgument,
        ),
        (
            AppError::Unprocessable("bad".to_string()),
            Code::InvalidArgument,
        ),
        (AppError::NotFound("none".to_string()), Code::NotFound),
        (AppError::Pool("refused".to_string()), Code::Unavailable),
        (AppError::Unavailable("down".to_string()), Code::Unavailable),
        (AppError::Internal("broken".to_string()), Code::Internal),
    ];

    for (error, expected) in cases {
        let shown = format!("{error}");
        assert_eq!(
            status_of(error, "CalculatePrice").code(),
            expected,
            "{shown}"
        );
    }
}

#[tokio::test]
async fn a_cart_whose_lines_add_up_past_what_money_carries_is_refused_rather_than_priced_as_zero() {
    for items in [
        vec![line(2, i64::MAX)],
        vec![line(i32::MAX, i64::MAX / 2)],
        (0..1_000)
            .map(|_| return line(i32::MAX, i64::MAX))
            .collect(),
    ] {
        let lines = items.len();
        let price = server()
            .calculate_price(Request::new(CalculatePriceRequest {
                items,
                ..Default::default()
            }))
            .await;

        assert_eq!(
            price.err().map(|status| return status.code()),
            Some(Code::InvalidArgument),
            "{lines} line(s)"
        );
    }
}
