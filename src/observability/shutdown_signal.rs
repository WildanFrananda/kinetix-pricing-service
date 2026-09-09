use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::watch;

#[derive(Clone)]
pub struct ShutdownSignal {
    fired: watch::Sender<bool>,
}

impl ShutdownSignal {
    pub fn new() -> Self {
        let (fired, _) = watch::channel(false);
        return ShutdownSignal { fired };
    }

    pub fn listen_for_signals(&self) -> Result<(), String> {
        let mut terminate = signal(SignalKind::terminate())
            .map_err(|error| format!("cannot listen for SIGTERM: {error}"))?;
        let mut interrupt = signal(SignalKind::interrupt())
            .map_err(|error| format!("cannot listen for SIGINT: {error}"))?;

        let signalled = self.clone();
        tokio::spawn(async move {
            let reason = tokio::select! {
                _ = terminate.recv() => "SIGTERM",
                _ = interrupt.recv() => "SIGINT",
            };
            signalled.trigger(reason);
        });

        return Ok(());
    }

    pub fn trigger(&self, reason: &str) {
        let first = self.fired.send_if_modified(|fired| {
            if *fired {
                return false;
            }
            *fired = true;
            return true;
        });

        if first {
            tracing::info!(
                reason = reason,
                "shutdown requested: taking no new work and finishing what is in flight"
            );
        }
        return;
    }

    pub async fn wait(&self) {
        let mut receiver = self.fired.subscribe();
        let _ = receiver.wait_for(|fired| *fired).await;
        return;
    }
}

impl Default for ShutdownSignal {
    fn default() -> Self {
        return ShutdownSignal::new();
    }
}
