pub mod shipping_proto {
    tonic::include_proto!("shipping.v1");
}

use rust_decimal::Decimal;
use std::str::FromStr;
use shipping_proto::shipping_service_client::ShippingServiceClient;
use shipping_proto::{EstimateShippingOptionsRequest, LocationCoordinates};

#[derive(Debug, Clone)]
pub struct ShippingOptionResult {
    pub service_tier: String,
    pub service_name: String,
    pub distance_km: f64,
    pub base_shipping_fee: Decimal,
    pub estimated_delivery_time: String,
    pub is_available: bool,
    pub unavailable_reason: String,
}

#[derive(Debug, Clone)]
pub struct EstimateShippingResult {
    pub distance_km: f64,
    pub options: Vec<ShippingOptionResult>,
}

pub struct ShippingGrpcClient {
    pub endpoint_url: String,
}

impl ShippingGrpcClient {
    pub fn new(endpoint_url: String) -> Self {
        return Self { endpoint_url };
    }

    pub async fn estimate_shipping_options(
        &self,
        origin_lat: f64,
        origin_lng: f64,
        dest_lat: f64,
        dest_lng: f64,
        total_weight_kg: f64,
        merchant_id: Option<i64>,
    ) -> Result<EstimateShippingResult, String> {
        let mut client = ShippingServiceClient::connect(self.endpoint_url.clone())
            .await
            .map_err(|e| return format!("Failed to connect to matching gRPC: {}", e))?;

        let request = tonic::Request::new(EstimateShippingOptionsRequest {
            origin: Some(LocationCoordinates {
                latitude: origin_lat,
                longitude: origin_lng,
            }),
            destination: Some(LocationCoordinates {
                latitude: dest_lat,
                longitude: dest_lng,
            }),
            total_weight_kg,
            merchant_id: merchant_id.unwrap_or(0),
        });

        let response = client
            .estimate_shipping_options(request)
            .await
            .map_err(|e| return format!("gRPC EstimateShippingOptions error: {}", e))?
            .into_inner();

        let options = response
            .options
            .into_iter()
            .map(|opt| {
                let base_fee = Decimal::from_str(&opt.base_shipping_fee.to_string())
                    .unwrap_or(Decimal::ZERO);

                return ShippingOptionResult {
                    service_tier: opt.service_tier,
                    service_name: opt.service_name,
                    distance_km: opt.distance_km,
                    base_shipping_fee: base_fee,
                    estimated_delivery_time: opt.estimated_delivery_time,
                    is_available: opt.is_available,
                    unavailable_reason: opt.unavailable_reason,
                };
            })
            .collect();

        return Ok(EstimateShippingResult {
            distance_km: response.distance_km,
            options,
        });
    }
}
