use rocket::http::Method;
use rocket::Request;

pub fn allowed_methods(request: &Request<'_>) -> Vec<Method> {
    let wanted: Vec<&str> = request.uri().path().segments().collect();
    let mut methods: Vec<Method> = Vec::new();

    for route in request.rocket().routes() {
        if template_matches(route.uri.path(), &wanted) && !methods.contains(&route.method) {
            methods.push(route.method);
        }
    }

    if methods.contains(&Method::Get) && !methods.contains(&Method::Head) {
        methods.push(Method::Head);
    }

    methods.sort_by_key(|method| return method.as_str());
    return methods;
}

fn template_matches(template: &str, wanted: &[&str]) -> bool {
    let segments: Vec<&str> = template
        .split('/')
        .filter(|segment| return !segment.is_empty())
        .collect();

    if let Some((last, fixed)) = segments.split_last() {
        if last.starts_with('<') && last.ends_with("..>") {
            return wanted.len() >= fixed.len() && all_match(fixed, wanted);
        }
    }

    return segments.len() == wanted.len() && all_match(&segments, wanted);
}

fn all_match(template: &[&str], wanted: &[&str]) -> bool {
    return template.iter().zip(wanted).all(|(expected, actual)| {
        let dynamic = expected.starts_with('<') && expected.ends_with('>');
        return dynamic || expected == actual;
    });
}
