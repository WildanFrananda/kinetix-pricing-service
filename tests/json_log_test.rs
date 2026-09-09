use std::io;
use std::sync::{Arc, Mutex};

use serde_json::Value;
use tracing_subscriber::fmt::MakeWriter;

use kinetix_pricing_service::observability::json_logging;

const CORRELATION_ID: &str = "kinetix-trace-1757400000-31337";

#[derive(Clone)]
struct SharedBuffer(Arc<Mutex<Vec<u8>>>);

struct SharedBufferWriter(Arc<Mutex<Vec<u8>>>);

impl<'writer> MakeWriter<'writer> for SharedBuffer {
    type Writer = SharedBufferWriter;

    fn make_writer(&'writer self) -> Self::Writer {
        return SharedBufferWriter(self.0.clone());
    }
}

impl io::Write for SharedBufferWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0
            .lock()
            .expect("the log buffer was poisoned")
            .extend_from_slice(buf);
        return Ok(buf.len());
    }

    fn flush(&mut self) -> io::Result<()> {
        return Ok(());
    }
}

fn captured(emit: impl FnOnce()) -> String {
    let buffer = Arc::new(Mutex::new(Vec::new()));
    let subscriber = json_logging::subscriber(SharedBuffer(buffer.clone()));

    tracing::subscriber::with_default(subscriber, emit);

    let bytes = buffer.lock().expect("the log buffer was poisoned").clone();
    return String::from_utf8(bytes).expect("the log line is not UTF-8");
}

#[test]
fn a_log_line_is_one_json_object_per_event() {
    let output = captured(|| {
        tracing::info!(
            peer = "kinetix-order-service",
            request_id = CORRELATION_ID,
            "gRPC call accepted"
        );
        tracing::warn!(request_id = CORRELATION_ID, "and a second event");
    });

    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(
        lines.len(),
        2,
        "two events did not make two lines:\n{output}"
    );

    for line in lines {
        serde_json::from_str::<Value>(line).expect("a log line is not a JSON object");
    }
}

#[test]
fn the_required_fields_are_at_the_top_level_where_a_pipeline_reads_them() {
    let output = captured(|| {
        tracing::info!(
            peer = "kinetix-order-service",
            request_id = CORRELATION_ID,
            "gRPC call accepted"
        );
    });

    let line: Value = serde_json::from_str(output.trim()).expect("the log line is not JSON");

    assert!(line["timestamp"].is_string(), "no timestamp: {line}");
    assert_eq!(line["level"], "INFO");
    assert_eq!(line["message"], "gRPC call accepted");
    assert_eq!(
        line["target"], "json_log_test",
        "no logger/source on the line: {line}"
    );
    assert_eq!(line["request_id"], CORRELATION_ID);

    assert_eq!(line["peer"], "kinetix-order-service");

    assert!(
        line.get("fields").is_none(),
        "the event is still nested: {line}"
    );
}

#[test]
fn the_correlation_id_is_still_a_bare_substring_of_the_line() {
    let output = captured(|| {
        tracing::info!(request_id = CORRELATION_ID, "HTTP request");
    });

    assert!(
        output.contains(CORRELATION_ID),
        "the id did not survive the encoder as a bare substring:\n{output}"
    );
}

#[test]
fn an_id_with_json_metacharacters_would_be_escaped_and_the_test_would_say_so() {
    let quoted = r#"kinetix-trace-"quoted""#;
    let output = captured(|| {
        tracing::info!(request_id = quoted, "HTTP request");
    });

    assert!(
        !output.contains(quoted),
        "a quote in an id passed through the encoder untouched, which serde_json does not do"
    );
    assert!(
        CORRELATION_ID
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-'),
        "the ids this estate generates are no longer plain enough to grep for"
    );
}
