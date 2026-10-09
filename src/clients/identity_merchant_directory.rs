use std::time::Duration;

use async_trait::async_trait;
use tonic::transport::Channel;
use tonic::Request;

use super::ports::{MerchantDirectoryPort, MerchantStanding, PeerUnavailable};
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
    async fn standing_of(
        &self,
        principal_id: &str,
    ) -> Result<Option<MerchantStanding>, PeerUnavailable> {
        let mut request = Request::new(GetMerchantInfoRequest {
            principal_id: principal_id.to_string(),
        });
        request.set_timeout(DEADLINE);

        let response = self
            .client
            .clone()
            .get_merchant_info(request)
            .await
            .map_err(|status| {
                return PeerUnavailable(format!(
                    "identity did not answer GetMerchantInfo: {}",
                    status.message()
                ));
            })?
            .into_inner();

        if !response.found {
            return Ok(None);
        }

        return Ok(Some(MerchantStanding {
            merchant_principal_id: response.merchant_principal_id,
            may_sell: response.may_sell,
        }));
    }
}
