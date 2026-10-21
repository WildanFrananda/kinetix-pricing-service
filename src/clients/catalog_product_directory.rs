use std::time::Duration;

use async_trait::async_trait;
use tonic::transport::Channel;
use tonic::{Code, Request};

use super::peer_error::PeerError;
use super::product_directory_port::ProductDirectoryPort;
use crate::proto::catalog::v1::catalog_service_client::CatalogServiceClient;
use crate::proto::catalog::v1::GetProductRequest;

const DEADLINE: Duration = Duration::from_secs(5);

pub struct CatalogProductDirectory {
    client: CatalogServiceClient<Channel>,
}

impl CatalogProductDirectory {
    pub fn new(channel: Channel) -> Self {
        return Self {
            client: CatalogServiceClient::new(channel),
        };
    }
}

#[async_trait]
impl ProductDirectoryPort for CatalogProductDirectory {
    async fn owner_of(&self, sku: &str) -> Result<Option<String>, PeerError> {
        let mut request = Request::new(GetProductRequest {
            sku: sku.to_string(),
        });
        request.set_timeout(DEADLINE);

        let response = match self.client.clone().get_product(request).await {
            Ok(response) => response.into_inner(),
            Err(status) if status.code() == Code::NotFound => return Ok(None),
            Err(status) => {
                return Err(PeerError::from_status("catalog", "GetProduct", &status));
            }
        };

        if !response.found {
            return Ok(None);
        }

        return Ok(response
            .product
            .map(|product| return product.merchant_principal_id));
    }
}
