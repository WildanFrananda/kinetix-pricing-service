use std::time::Instant;

pub struct RequestStart(pub Instant);

impl RequestStart {
    pub fn now() -> Self {
        return RequestStart(Instant::now());
    }
}
