#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuotaOutcome {
    Applied { remaining: i32 },
    AlreadyDone { remaining: i32 },
    Exhausted,
    Expired,
    NotFound,
    NotForThisMerchant,
}
