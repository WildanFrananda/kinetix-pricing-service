#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BearerFailure {
    Absent,
    Invalid,
    Unavailable(String),
    Unconfigured,
}
