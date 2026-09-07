use std::thread::sleep;
use std::time::{Duration, Instant};

use kinetix_pricing_service::security::jwks_breaker::JwksBreaker;

#[test]
fn a_fresh_breaker_lets_the_first_call_through() {
    let breaker = JwksBreaker::new(Duration::from_secs(15));

    assert!(breaker.cooldown_remaining().is_none());
}

#[test]
fn one_failure_is_enough_to_open_the_circuit() {
    let breaker = JwksBreaker::new(Duration::from_secs(15));

    breaker.record_failure();

    let remaining = breaker
        .cooldown_remaining()
        .expect("the circuit should be open");
    assert!(remaining <= Duration::from_secs(15));
    assert!(remaining > Duration::from_secs(14));
}

#[test]
fn the_circuit_closes_by_itself_once_the_cooldown_elapses() {
    let breaker = JwksBreaker::new(Duration::from_millis(30));

    breaker.record_failure();
    assert!(breaker.cooldown_remaining().is_some());

    sleep(Duration::from_millis(60));

    assert!(breaker.cooldown_remaining().is_none());
}

#[test]
fn a_success_closes_the_circuit_early() {
    let breaker = JwksBreaker::new(Duration::from_secs(15));

    breaker.record_failure();
    breaker.record_success();

    assert!(breaker.cooldown_remaining().is_none());
}

#[test]
fn only_a_success_that_lands_after_the_mark_serves_a_queued_caller() {
    let breaker = JwksBreaker::new(Duration::from_secs(15));

    breaker.record_success();
    let queued_at = Instant::now();

    assert!(!breaker.refreshed_since(queued_at));

    breaker.record_success();

    assert!(breaker.refreshed_since(queued_at));
}
