use tonic::transport::{Channel, Endpoint};

use crate::security::ServiceIdentity;

pub fn mesh_channel(url: &str, identity: &ServiceIdentity) -> Result<Channel, String> {
    let address = url
        .strip_prefix("https://")
        .or_else(|| return url.strip_prefix("http://"))
        .unwrap_or(url)
        .trim_end_matches('/');
    let host = address.split(':').next().unwrap_or_default();

    if host.is_empty() {
        return Err(format!("{url} names no host"));
    }

    let endpoint = Endpoint::from_shared(format!("https://{address}"))
        .map_err(|e| return format!("{url} is not a usable address: {e}"))?
        .tls_config(identity.client_tls(host))
        .map_err(|e| return format!("cannot set up TLS to {url}: {e}"))?;

    return Ok(endpoint.connect_lazy());
}
