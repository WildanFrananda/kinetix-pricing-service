use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

pub struct JwksBreaker {
    cooldown: Duration,
    open_until: Mutex<Option<Instant>>,
    last_success: Mutex<Option<Instant>>,
}

impl JwksBreaker {
    pub fn new(cooldown: Duration) -> Self {
        return Self {
            cooldown,
            open_until: Mutex::new(None),
            last_success: Mutex::new(None),
        };
    }

    pub fn cooldown_remaining(&self) -> Option<Duration> {
        let open_until = *lock(&self.open_until);
        match open_until {
            Some(until) => return until.checked_duration_since(Instant::now()),
            None => return None,
        }
    }

    pub fn refreshed_since(&self, mark: Instant) -> bool {
        let last_success = *lock(&self.last_success);
        match last_success {
            Some(at) => return at > mark,
            None => return false,
        }
    }

    pub fn record_success(&self) {
        *lock(&self.last_success) = Some(Instant::now());
        *lock(&self.open_until) = None;
    }

    pub fn record_failure(&self) {
        *lock(&self.open_until) = Some(Instant::now() + self.cooldown);
    }
}

fn lock<T>(cell: &Mutex<T>) -> MutexGuard<'_, T> {
    return cell.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
}
