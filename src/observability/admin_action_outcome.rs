use rocket::http::Status;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdminActionOutcome {
    Done,
    Refused,
    Failed,
}

impl AdminActionOutcome {
    pub const ALL: [AdminActionOutcome; 3] = [
        AdminActionOutcome::Done,
        AdminActionOutcome::Refused,
        AdminActionOutcome::Failed,
    ];

    pub fn for_status(status: Status) -> Self {
        if status.code < 400 {
            return AdminActionOutcome::Done;
        }

        if status.code < 500 {
            return AdminActionOutcome::Refused;
        }

        return AdminActionOutcome::Failed;
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            AdminActionOutcome::Done => return "done",
            AdminActionOutcome::Refused => return "refused",
            AdminActionOutcome::Failed => return "failed",
        }
    }
}
