use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreationClaim {
    Fresh,
    Replay(Uuid),
}
