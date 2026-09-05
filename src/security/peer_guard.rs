use tonic::{Request, Status};

use super::spiffe::peer_service;

#[derive(Clone)]
pub struct PeerGuard {
    allowed: Vec<String>,
}

impl PeerGuard {
    pub fn from_env() -> Result<Self, String> {
        let raw = std::env::var("KINETIX_GRPC_ALLOWED_PEERS")
            .map_err(|_| "KINETIX_GRPC_ALLOWED_PEERS is required and has no default".to_string())?;

        let allowed: Vec<String> = raw
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if allowed.is_empty() {
            return Err("KINETIX_GRPC_ALLOWED_PEERS is set but names no services".to_string());
        }
        Ok(Self { allowed })
    }

    pub fn check<T>(&self, request: Request<T>) -> Result<Request<T>, Status> {
        let peer = match peer_service(&request) {
            Some(p) => p,
            None => {
                return Err(Status::unauthenticated(
                    "a client certificate carrying a SPIFFE identity is required",
                ))
            }
        };

        if !self.allowed.iter().any(|a| a == &peer) {
            tracing::warn!(peer = %peer, "refused a gRPC call from a service that is not on the allow list");
            return Err(Status::permission_denied("this service is not permitted to call pricing"));
        }

        Ok(request)
    }
}
