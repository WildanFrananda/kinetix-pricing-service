use kinetix_pricing_service::observability::Metrics;

fn metrics() -> Metrics {
    return Metrics::new("kinetix-pricing-service", "0.1.0").expect("the registry was refused");
}

#[test]
fn every_metric_the_contract_names_is_on_the_body_before_any_traffic() {
    let metrics = metrics();
    metrics.seed_http_route("GET", "/health/ready");
    let body = metrics.encode().expect("the registry would not encode");

    for name in [
        "kinetix_http_requests_total",
        "kinetix_http_request_duration_seconds_bucket",
        "kinetix_http_request_duration_seconds_sum",
        "kinetix_http_request_duration_seconds_count",
        "kinetix_grpc_server_calls_total",
        "kinetix_build_info",
    ] {
        assert!(
            body.lines().any(|line| line.starts_with(name)),
            "no line starts with {name}:\n{body}"
        );
    }
}

#[test]
fn build_info_names_the_service_and_the_version_and_nothing_else() {
    let body = metrics().encode().expect("the registry would not encode");
    let line = body
        .lines()
        .find(|line| line.starts_with("kinetix_build_info"))
        .expect("no kinetix_build_info series");

    assert!(
        line.contains(r#"service="kinetix-pricing-service""#),
        "{line}"
    );
    assert!(line.contains(r#"version="0.1.0""#), "{line}");
    assert!(line.ends_with(" 1"), "{line}");
}

#[test]
fn service_is_a_label_on_build_info_alone() {
    let metrics = metrics();
    metrics.observe_http("GET", "/health", 200, 0.001);
    metrics.observe_grpc_server_call("pricing.v1.PricingService/CalculatePrice", "OK");
    let body = metrics.encode().expect("the registry would not encode");

    for line in body.lines() {
        if line.starts_with('#') || line.starts_with("kinetix_build_info") {
            continue;
        }
        assert!(
            !line.contains("service=\""),
            "service is repeated off build_info, where the scrape target already says it: {line}"
        );
    }
}

#[test]
fn a_duration_is_recorded_in_seconds() {
    let metrics = metrics();
    metrics.observe_http("GET", "/health", 200, 0.25);
    let body = metrics.encode().expect("the registry would not encode");

    assert!(
        body.lines().any(
            |line| line.starts_with("kinetix_http_request_duration_seconds_sum")
                && line.ends_with(" 0.25")
        ),
        "the histogram sum is not the seconds that were passed in:\n{body}"
    );
}
