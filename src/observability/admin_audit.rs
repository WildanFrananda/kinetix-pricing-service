use super::admin_action_outcome::AdminActionOutcome;
use super::admin_actions::ADMIN_ACTION_EVENT;
use super::metrics::Metrics;

pub fn record_admin_action(
    metrics: &Metrics,
    actor: &str,
    action: &str,
    target: &str,
    outcome: AdminActionOutcome,
    request_id: &str,
) {
    tracing::info!(
        target: ADMIN_ACTION_EVENT,
        event = ADMIN_ACTION_EVENT,
        actor,
        action,
        target,
        outcome = outcome.as_str(),
        request_id,
        "admin_action"
    );
    metrics.observe_admin_action(action, outcome);
    return;
}
