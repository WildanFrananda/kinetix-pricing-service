use std::time::Duration;

use async_trait::async_trait;
use tonic::transport::Channel;
use tonic::{Code, Request};

use super::merchant_directory_port::MerchantDirectoryPort;
use super::merchant_standing::MerchantStanding;
use super::peer_error::PeerError;
use crate::proto::identity::v1::identity_service_client::IdentityServiceClient;
use crate::proto::identity::v1::GetMerchantInfoRequest;

const DEADLINE: Duration = Duration::from_secs(5);

pub struct IdentityMerchantDirectory {
    client: IdentityServiceClient<Channel>,
}

impl IdentityMerchantDirectory {
    pub fn new(channel: Channel) -> Self {
        return Self {
            client: IdentityServiceClient::new(channel),
        };
    }
}

#[async_trait]
impl MerchantDirectoryPort for IdentityMerchantDirectory {
    async fn standing_of(&self, principal_id: &str) -> Result<Option<MerchantStanding>, PeerError> {
        let mut request = Request::new(GetMerchantInfoRequest {
            principal_id: principal_id.to_string(),
        });
        request.set_timeout(DEADLINE);

        let response = match self.client.clone().get_merchant_info(request).await {
            Ok(response) => response.into_inner(),
            Err(status) if status.code() == Code::NotFound => return Ok(None),
            Err(status) => {
                return Err(PeerError::from_status(
                    "identity",
                    "GetMerchantInfo",
                    &status,
                ));
            }
        };

        if !response.found {
            return Ok(None);
        }

        return Ok(Some(MerchantStanding {
            merchant_principal_id: response.merchant_principal_id,
            may_sell: response.may_sell,
        }));
    }
}
