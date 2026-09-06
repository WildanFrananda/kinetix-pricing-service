use rust_decimal::Decimal;
use tonic::transport::{Channel, Endpoint};

use crate::money::from_money;
use crate::proto::common::v1::GeoPoint;
use crate::proto::shipping::v1 as shipping_proto;
use crate::security::ServiceIdentity;
use shipping_proto::shipping_service_client::ShippingServiceClient;
use shipping_proto::EstimateShippingOptionsRequest;

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
        _merchant_id: Option<i64>,
    ) -> Result<EstimateShippingResult, String> {
        let identity = ServiceIdentity::load()
            .map_err(|e| return format!("cannot present a client certificate to matching: {}", e))?;

        let authority = self
            .endpoint_url
            .trim_start_matches("http://")
            .trim_start_matches("https://")
            .split(':')
            .next()
            .unwrap_or("kinetix-matching-service")
            .to_string();

        let endpoint = Endpoint::from_shared(self.endpoint_url.clone())
            .map_err(|e| return format!("matching endpoint is not a valid URL: {}", e))?
            .tls_config(identity.client_tls(&authority))
            .map_err(|e| return format!("client TLS could not be configured: {}", e))?;

        let channel: Channel = endpoint
            .connect()
            .await
            .map_err(|e| return format!("Failed to connect to matching gRPC: {}", e))?;

        let mut client = ShippingServiceClient::new(channel);

        let request = tonic::Request::new(EstimateShippingOptionsRequest {
            origin: Some(GeoPoint {
                latitude: origin_lat,
                longitude: origin_lng,
            }),
            destination: Some(GeoPoint {
                latitude: dest_lat,
                longitude: dest_lng,
            }),
            total_weight_grams: (total_weight_kg * 1000.0).round() as i64,
            merchant_principal_id: String::new(),
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
                let base_fee = opt
                    .base_shipping_fee
                    .as_ref()
                    .and_then(|m| from_money(m).ok())
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
