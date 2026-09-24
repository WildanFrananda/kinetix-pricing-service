use std::sync::OnceLock;

use tonic::Request;
use x509_parser::prelude::*;

const DEFAULT_TRUST_DOMAIN: &str = "kinetix.local";

pub fn trust_domain() -> &'static str {
    static CONFIGURED: OnceLock<String> = OnceLock::new();

    return CONFIGURED
        .get_or_init(|| {
            return std::env::var("KINETIX_TRUST_DOMAIN")
                .ok()
                .map(|value| return value.trim().to_string())
                .filter(|value| return !value.is_empty())
                .unwrap_or_else(|| return DEFAULT_TRUST_DOMAIN.to_string());
        })
        .as_str();
}

pub fn service_of(id: &str, domain: &str) -> Option<String> {
    return id
        .strip_prefix(&format!("spiffe://{domain}/service/"))
        .map(|name| return name.to_string());
}

pub fn peer_spiffe_id<T>(request: &Request<T>) -> Option<String> {
    let certs = request.peer_certs()?;
    let leaf = certs.first()?;
    let expected = format!("spiffe://{}/", trust_domain());

    let (_, parsed) = X509Certificate::from_der(leaf.get_ref()).ok()?;
    for ext in parsed.extensions() {
        if let ParsedExtension::SubjectAlternativeName(san) = ext.parsed_extension() {
            for name in &san.general_names {
                if let GeneralName::URI(uri) = name {
                    if uri.starts_with(&expected) {
                        return Some((*uri).to_string());
                    }
                }
            }
        }
    }
    return None;
}

pub fn peer_service<T>(request: &Request<T>) -> Option<String> {
    let id = peer_spiffe_id(request)?;
    return service_of(&id, trust_domain());
}
