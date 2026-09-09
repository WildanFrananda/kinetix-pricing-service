use std::sync::Arc;

use rocket::http::Status;
use rocket::local::blocking::Client;
use rocket::{get, routes};

use kinetix_pricing_service::db::create_pool;
use kinetix_pricing_service::observability::{Metrics, MetricsFairing};
use kinetix_pricing_service::routes::{
    flash_sale_routes::get_flash_sale_for_product, health_routes::health_check,
    metrics_routes::metrics as metrics_endpoint, voucher_routes::get_voucher,
};
use kinetix_pricing_service::DbPool;

use crate::identifier_shapes::looks_like_an_identifier;

const VOUCHER_CODE: &str = "8f3a1c22-0e5b-4a91-9f0d-2b7c11d4e6aa";

#[get("/stub/vouchers/<code>")]
fn stub_voucher(code: &str) -> String {
    return code.to_string();
}

fn unconnected_pool() -> DbPool {
    return create_pool("postgres://unused:unused@127.0.0.1:1/unused");
}

fn client() -> Client {
    let metrics = Arc::new(
        Metrics::new("kinetix-pricing-service", "0.1.0").expect("the metric registry was refused"),
    );

    let rocket = rocket::build()
        .attach(MetricsFairing::new(metrics.clone()))
        .manage(unconnected_pool())
        .manage(metrics)
        .mount(
            "/",
            routes![
                health_check,
                metrics_endpoint,
                get_voucher,
                get_flash_sale_for_product,
                stub_voucher
            ],
        );

    return Client::tracked(rocket).expect("the test rocket did not build");
}

fn exposition(client: &Client) -> String {
    let response = client.get("/metrics").dispatch();
    assert_eq!(response.status(), Status::Ok);
    return response.into_string().expect("/metrics served no body");
}

#[test]
fn metrics_is_prometheus_text_and_needs_no_credential() {
    let client = client();
    let response = client.get("/metrics").dispatch();

    assert_eq!(response.status(), Status::Ok);

    let content_type = response
        .content_type()
        .expect("/metrics served no content type")
        .to_string();
    assert!(
        content_type.starts_with("text/plain"),
        "the exposition is not text/plain: {content_type}"
    );

    let body = response.into_string().expect("/metrics served no body");
    assert!(
        body.lines().any(|line| line.starts_with("# HELP ")),
        "no '# HELP' line, so this is not Prometheus text format:\n{body}"
    );
}

#[test]
fn every_metric_the_contract_names_for_pricing_is_served() {
    let body = exposition(&client());

    for name in [
        "kinetix_http_requests_total",
        "kinetix_http_request_duration_seconds",
        "kinetix_grpc_server_calls_total",
        "kinetix_build_info",
    ] {
        assert!(
            body.lines().any(|line| line.starts_with(name)),
            "no series line starts with {name}:\n{body}"
        );
    }
}

#[test]
fn a_route_label_is_the_template_and_never_the_path_that_was_dialled() {
    let client = client();

    let path = format!("/stub/vouchers/{VOUCHER_CODE}");
    assert_eq!(client.get(path).dispatch().status(), Status::Ok);

    let body = exposition(&client);

    assert!(
        body.contains(r#"route="/stub/vouchers/{code}""#),
        "the matched route did not reach the label as a template:\n{body}"
    );
    assert!(
        !body.contains(VOUCHER_CODE),
        "the code from the path is in the exposition:\n{body}"
    );
}

#[test]
fn the_real_dynamic_routes_are_seeded_as_templates_before_anyone_calls_them() {
    let body = exposition(&client());

    for template in [
        r#"route="/api/v1/vouchers/{code}""#,
        r#"route="/api/v1/flash-sales/{product_id}""#,
    ] {
        assert!(
            body.contains(template),
            "{template} is not on /metrics, so a dashboard sees the route only once somebody has \
             called it:\n{body}"
        );
    }
}

#[test]
fn a_path_no_route_claims_is_counted_under_one_bounded_label() {
    let client = client();

    for probe in ["/.env", "/admin", "/wp-login.php"] {
        assert_eq!(client.get(probe).dispatch().status(), Status::NotFound);
    }

    let body = exposition(&client);

    assert!(
        body.contains(r#"route="unmatched""#),
        "unrouted requests were not counted:\n{body}"
    );
    for probe in [".env", "admin", "wp-login"] {
        assert!(
            !body.contains(probe),
            "the probed path {probe} became a label, and so would every future guess:\n{body}"
        );
    }
}

#[test]
fn no_label_anywhere_carries_an_identifier() {
    let client = client();
    let path = format!("/stub/vouchers/{VOUCHER_CODE}");
    client.get(path).dispatch();
    client.get("/health").dispatch();

    let body = exposition(&client);

    for line in body.lines() {
        if line.starts_with('#') {
            continue;
        }
        for value in label_values(line) {
            assert!(
                !looks_like_an_identifier(value),
                "an identifier reached a metric label: {line}"
            );
        }
    }
}

#[test]
fn the_status_and_the_duration_are_both_recorded_for_a_real_request() {
    let client = client();
    client.get("/health").dispatch();

    let body = exposition(&client);

    assert!(
        body.lines().any(|line| {
            return line.starts_with(
                r#"kinetix_http_requests_total{method="GET",route="/health",status="200"}"#,
            ) && line.ends_with(" 1");
        }),
        "the /health request was not counted:\n{body}"
    );

    let sum = body
        .lines()
        .find(|line| {
            return line.starts_with(
                r#"kinetix_http_request_duration_seconds_sum{method="GET",route="/health"}"#,
            );
        })
        .and_then(|line| line.rsplit(' ').next())
        .expect("no duration sum for /health")
        .parse::<f64>()
        .expect("the duration sum is not a number");

    assert!(sum > 0.0, "the duration for /health was recorded as {sum}");
}

fn label_values(line: &str) -> Vec<&str> {
    let mut values = Vec::new();
    let Some(open) = line.find('{') else {
        return values;
    };
    let Some(close) = line.rfind('}') else {
        return values;
    };

    let mut rest = &line[open + 1..close];
    while let Some(quote) = rest.find('"') {
        rest = &rest[quote + 1..];
        match rest.find('"') {
            Some(end) => {
                values.push(&rest[..end]);
                rest = &rest[end + 1..];
            }
            None => break,
        }
    }
    return values;
}

mod identifier_shapes {
    pub fn looks_like_an_identifier(value: &str) -> bool {
        return looks_like_a_uuid(value) || has_a_long_hex_run(value) || looks_like_an_email(value);
    }

    fn looks_like_a_uuid(value: &str) -> bool {
        let bytes = value.as_bytes();
        if bytes.len() < 14 {
            return false;
        }
        for start in 0..=bytes.len() - 14 {
            let hyphenated = bytes[start..start + 14]
                .iter()
                .enumerate()
                .all(|(index, byte)| {
                    return match index {
                        8 | 13 => *byte == b'-',
                        _ => byte.is_ascii_hexdigit(),
                    };
                });
            if hyphenated {
                return true;
            }
        }
        return false;
    }

    fn has_a_long_hex_run(value: &str) -> bool {
        let mut run = 0;
        for byte in value.as_bytes() {
            run = if byte.is_ascii_hexdigit() { run + 1 } else { 0 };
            if run >= 24 {
                return true;
            }
        }
        return false;
    }

    fn looks_like_an_email(value: &str) -> bool {
        let Some((_, domain)) = value.split_once('@') else {
            return false;
        };
        return domain.contains('.')
            && domain
                .split('.')
                .next_back()
                .is_some_and(|tld| tld.len() >= 2);
    }
}
