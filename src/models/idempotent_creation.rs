#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdempotentCreation {
    pub principal_id: String,
    pub idempotency_key: String,
}
