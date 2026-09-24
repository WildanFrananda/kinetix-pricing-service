use std::sync::OnceLock;

use tonic::Request;
use x509_parser::prelude::*;

const DEFAULT_TRUST_DOMAIN: &str = "kinetix.local";

pub fn trust_domains() -> &'static [String] {
    static CONFIGURED: OnceLock<Vec<String>> = OnceLock::new();

    return CONFIGURED.get_or_init(|| {
        let configured: Vec<String> = std::env::var("KINETIX_TRUST_DOMAIN")
            .unwrap_or_default()
            .split(',')
            .map(|value| return value.trim().to_string())
            .filter(|value| return !value.is_empty())
            .collect();

        if configured.is_empty() {
            return vec![DEFAULT_TRUST_DOMAIN.to_string()];
        }

        return configured;
    });
}

pub fn trust_domain() -> &'static str {
    return trust_domains()[0].as_str();
}

pub fn service_of(id: &str, domain: &str) -> Option<String> {
    return id
        .strip_prefix(&format!("spiffe://{domain}/service/"))
        .map(|name| return name.to_string());
}

pub fn peer_spiffe_id<T>(request: &Request<T>) -> Option<String> {
    let certs = request.peer_certs()?;
    let leaf = certs.first()?;
    let accepted: Vec<String> = trust_domains()
        .iter()
        .map(|domain| return format!("spiffe://{domain}/"))
        .collect();

    let (_, parsed) = X509Certificate::from_der(leaf.get_ref()).ok()?;
    for ext in parsed.extensions() {
        if let ParsedExtension::SubjectAlternativeName(san) = ext.parsed_extension() {
            for name in &san.general_names {
                if let GeneralName::URI(uri) = name {
                    if accepted.iter().any(|prefix| return uri.starts_with(prefix)) {
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

    for domain in trust_domains() {
        if let Some(name) = service_of(&id, domain) {
            return Some(name);
        }
    }

    return None;
}
