use std::io;

use tracing::level_filters::LevelFilter;
use tracing::Subscriber;
use tracing_subscriber::filter::Targets;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter, Layer};

use super::admin_actions::ADMIN_ACTION_EVENT;

pub fn subscriber<W>(writer: W) -> impl Subscriber + Send + Sync + 'static
where
    W: for<'writer> MakeWriter<'writer> + Clone + Send + Sync + 'static,
{
    let everything_but_admin_actions = Targets::new()
        .with_default(LevelFilter::TRACE)
        .with_target(ADMIN_ACTION_EVENT, LevelFilter::OFF);
    let service_events = fmt::layer()
        .json()
        .flatten_event(true)
        .with_current_span(false)
        .with_span_list(false)
        .with_writer(writer.clone())
        .with_filter(everything_but_admin_actions)
        .with_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")));

    let only_admin_actions = Targets::new().with_target(ADMIN_ACTION_EVENT, LevelFilter::INFO);
    let admin_actions = fmt::layer()
        .json()
        .flatten_event(true)
        .with_current_span(false)
        .with_span_list(false)
        .with_target(false)
        .with_writer(writer)
        .with_filter(only_admin_actions);

    return tracing_subscriber::registry()
        .with(service_events)
        .with(admin_actions);
}

pub fn init() {
    subscriber(io::stdout).init();
    return;
}
