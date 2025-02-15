use axum::body::Body;

pub(crate) fn index_html(index: &str, script: &str, body: &str) -> Body {
    Body::from(format!(include_str!("../templates/index.html"), index, script, body))
}

pub(crate) fn error_html(script: &str, body: &str) -> Body {
    Body::from(format!(include_str!("../templates/error.html"), script, body))
}
