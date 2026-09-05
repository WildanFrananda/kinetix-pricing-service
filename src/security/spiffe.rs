use tonic::Request;
use x509_parser::prelude::*;

const TRUST_DOMAIN: &str = "kinetix.local";

pub fn peer_spiffe_id<T>(request: &Request<T>) -> Option<String> {
    let certs = request.peer_certs()?;
    let leaf = certs.first()?;

    let (_, parsed) = X509Certificate::from_der(leaf.get_ref()).ok()?;
    for ext in parsed.extensions() {
        if let ParsedExtension::SubjectAlternativeName(san) = ext.parsed_extension() {
            for name in &san.general_names {
                if let GeneralName::URI(uri) = name {
                    if uri.starts_with(&format!("spiffe://{TRUST_DOMAIN}/")) {
                        return Some((*uri).to_string());
                    }
                }
            }
        }
    }
    None
}

pub fn peer_service<T>(request: &Request<T>) -> Option<String> {
    let id = peer_spiffe_id(request)?;
    id.strip_prefix(&format!("spiffe://{TRUST_DOMAIN}/service/"))
        .map(|s| s.to_string())
}
