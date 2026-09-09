pub fn route_template(rocket_uri: &str) -> String {
    let mut template = String::with_capacity(rocket_uri.len());
    let mut rest = rocket_uri;

    while let Some(open) = rest.find('<') {
        template.push_str(&rest[..open]);
        let after_open = &rest[open + 1..];

        match after_open.find('>') {
            Some(close) => {
                template.push('{');
                template.push_str(after_open[..close].trim_end_matches(".."));
                template.push('}');
                rest = &after_open[close + 1..];
            }
            None => {
                template.push_str(&rest[open..]);
                rest = "";
                break;
            }
        }
    }

    template.push_str(rest);
    return template;
}
