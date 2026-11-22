use axum::body::Body;
use axum::http::{header, HeaderMap, HeaderValue, Method, StatusCode};
use axum::response::Response;

use super::support::{error_response, is_not_modified, method_not_allowed, not_modified_response};
use crate::render;

const SITE_CSS: &str = include_str!("../../assets/site.css");
const SEARCH_JS: &str = include_str!("../../assets/search.js");
const VIEWER_JS: &str = include_str!("../../assets/viewer.js");

pub async fn serve_search_shell(method: Method, headers: HeaderMap) -> Response {
    match render::render_search_shell() {
        Ok(html) => embedded_response(method, &headers, &html, "text/html; charset=utf-8"),
        Err(error) => {
            eprintln!("rendering the embedded search shell failed: {error:#}");
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "rendering error")
        }
    }
}

pub async fn serve_site_css(method: Method, headers: HeaderMap) -> Response {
    embedded_response(method, &headers, SITE_CSS, "text/css; charset=utf-8")
}

pub async fn serve_search_js(method: Method, headers: HeaderMap) -> Response {
    embedded_response(
        method,
        &headers,
        SEARCH_JS,
        "text/javascript; charset=utf-8",
    )
}

pub async fn serve_viewer_js(method: Method, headers: HeaderMap) -> Response {
    embedded_response(
        method,
        &headers,
        VIEWER_JS,
        "text/javascript; charset=utf-8",
    )
}

fn embedded_response(
    method: Method,
    headers: &HeaderMap,
    content: &str,
    content_type: &'static str,
) -> Response {
    if !matches!(method, Method::GET | Method::HEAD) {
        return method_not_allowed();
    }

    let etag = content_etag(content.as_bytes());
    let cache_control = "no-cache";
    if is_not_modified(headers, &etag, None) {
        return not_modified_response(&etag, None, cache_control);
    }

    let length = content.len();
    let body = if method == Method::HEAD {
        Body::empty()
    } else {
        Body::from(content.to_string())
    };

    let mut response = Response::new(body);
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    headers.insert(
        header::CONTENT_LENGTH,
        HeaderValue::from_str(&length.to_string())
            .unwrap_or_else(|_| HeaderValue::from_static("0")),
    );
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(cache_control),
    );
    if let Ok(value) = HeaderValue::from_str(&etag) {
        headers.insert(header::ETAG, value);
    }
    response
}

/// Stable content validator for embedded assets; only changes when the bytes do.
fn content_etag(bytes: &[u8]) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("\"{hash:016x}\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_etags_change_with_content() {
        assert_eq!(content_etag(b"same"), content_etag(b"same"));
        assert_ne!(content_etag(b"first"), content_etag(b"second"));
    }
}
