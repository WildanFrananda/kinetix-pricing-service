#[derive(Debug, Clone)]
pub struct CreateOutcome<T> {
    pub record: T,
    pub replayed: bool,
}
