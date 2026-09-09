use kinetix_pricing_service::observability::route_template;

#[test]
fn a_static_route_is_its_own_template() {
    assert_eq!(route_template("/health/ready"), "/health/ready");
}

#[test]
fn a_dynamic_segment_becomes_a_braced_name() {
    assert_eq!(
        route_template("/api/v1/vouchers/<code>"),
        "/api/v1/vouchers/{code}"
    );
    assert_eq!(
        route_template("/api/v1/flash-sales/<product_id>"),
        "/api/v1/flash-sales/{product_id}"
    );
}

#[test]
fn trailing_segments_keep_the_name_and_lose_the_dots() {
    assert_eq!(route_template("/files/<path..>"), "/files/{path}");
}

#[test]
fn several_segments_are_all_translated() {
    assert_eq!(route_template("/a/<one>/b/<two>/c"), "/a/{one}/b/{two}/c");
}

#[test]
fn an_unterminated_segment_is_left_alone_rather_than_guessed_at() {
    assert_eq!(route_template("/api/v1/<broken"), "/api/v1/<broken");
}
