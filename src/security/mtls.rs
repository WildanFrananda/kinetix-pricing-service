use std::path::PathBuf;
use tonic::transport::{Certificate, ClientTlsConfig, Identity, ServerTlsConfig};

const DEFAULT_PKI_DIR: &str = "/pki";

pub struct ServiceIdentity {
    pub cert_pem: Vec<u8>,
    pub key_pem: Vec<u8>,
    pub ca_pem: Vec<u8>,
}

impl ServiceIdentity {
    pub fn load() -> Result<Self, String> {
        let dir: PathBuf = std::env::var("KINETIX_PKI_DIR")
            .unwrap_or_else(|_| return DEFAULT_PKI_DIR.to_string())
            .into();

        let read = |name: &str| -> Result<Vec<u8>, String> {
            let path = dir.join(name);
            return std::fs::read(&path).map_err(|e| {
                return format!(
                    "cannot read {}: {e}. The service PKI is mounted at {}; issue it with \
                     kinetix-infrastructure/bin/kinetix-pki issue.",
                    path.display(),
                    dir.display()
                );
            });
        };

        let cert_pem = read("tls.crt")?;
        let key_pem = read("tls.key")?;
        let ca_pem = read("ca.pem")?;

        for (name, bytes, marker) in [
            ("tls.crt", &cert_pem, "BEGIN CERTIFICATE"),
            ("tls.key", &key_pem, "PRIVATE KEY"),
            ("ca.pem", &ca_pem, "BEGIN CERTIFICATE"),
        ] {
            if !String::from_utf8_lossy(bytes).contains(marker) {
                return Err(format!(
                    "{}/{name} is not a PEM containing {marker}",
                    dir.display()
                ));
            }
        }

        return Ok(Self {
            cert_pem,
            key_pem,
            ca_pem,
        });
    }

    pub fn server_tls(&self) -> ServerTlsConfig {
        return ServerTlsConfig::new()
            .identity(Identity::from_pem(&self.cert_pem, &self.key_pem))
            .client_ca_root(Certificate::from_pem(&self.ca_pem));
    }

    pub fn client_tls(&self, domain_name: &str) -> ClientTlsConfig {
        return ClientTlsConfig::new()
            .identity(Identity::from_pem(&self.cert_pem, &self.key_pem))
            .ca_certificate(Certificate::from_pem(&self.ca_pem))
            .domain_name(domain_name);
    }
}
