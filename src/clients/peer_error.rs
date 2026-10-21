use tonic::{Code, Status};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeerError {
    Unavailable(String),
    Refused(String),
}

impl PeerError {
    pub fn from_status(peer: &str, rpc: &str, status: &Status) -> Self {
        match status.code() {
            Code::InvalidArgument => {
                return PeerError::Refused(format!("{peer} refused {rpc}: {}", status.message()));
            }
            code => {
                return PeerError::Unavailable(format!(
                    "{peer} did not answer {rpc}: {code:?}: {}",
                    status.message()
                ));
            }
        }
    }
}
