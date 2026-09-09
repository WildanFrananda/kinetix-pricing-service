use std::time::Duration;

use kinetix_pricing_service::observability::ShutdownSignal;

#[tokio::test]
async fn waiting_returns_once_the_signal_has_been_triggered() {
    let shutdown = ShutdownSignal::new();
    let waiter = shutdown.clone();
    let handle = tokio::spawn(async move { waiter.wait().await });

    shutdown.trigger("a test");

    tokio::time::timeout(Duration::from_secs(5), handle)
        .await
        .expect("wait() did not return after the signal fired")
        .expect("the waiting task panicked");
}

#[tokio::test]
async fn a_signal_that_already_fired_does_not_make_a_later_waiter_hang() {
    let shutdown = ShutdownSignal::new();
    shutdown.trigger("a test");

    tokio::time::timeout(Duration::from_secs(5), shutdown.wait())
        .await
        .expect("wait() blocked on a signal that had already fired");
}
