use axum::body::Body;
use axum::extract::State;
use axum::http::{header, HeaderMap, HeaderValue, Method, StatusCode, Uri};
use axum::response::Response;
use tokio_util::io::ReaderStream;

use super::support::{
    content_type_for, decode_segments, entity_tag, error_response, is_not_modified, last_modified,
    method_not_allowed, not_modified_response, resolve_data, resolve_error_response, Segments,
};
use super::AppState;

/// Serve a file from the published data root.
pub async fn serve_static(
    State(state): State<AppState>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
) -> Response {
    if !matches!(method, Method::GET | Method::HEAD) {
        return method_not_allowed();
    }

    let raw = uri.path().trim_start_matches('/');
    let decoded = if raw.is_empty() {
        Segments {
            parts: Vec::new(),
            trailing_slash: false,
        }
    } else {
        match decode_segments(raw, true) {
            Ok(decoded) => decoded,
            Err(()) => return error_response(StatusCode::BAD_REQUEST, "invalid path"),
        }
    };

    let mut segments = decoded.parts;
    if segments.is_empty() || decoded.trailing_slash {
        segments.push("index.html".to_string());
    }

    if segments
        .first()
        .map(|segment| segment == "state.sqlite" || segment == "build.lock")
        .unwrap_or(false)
    {
        return error_response(StatusCode::NOT_FOUND, "not found");
    }

    let path = match resolve_data(&state.data_root, &segments).await {
        Ok(path) => path,
        Err(error) => return resolve_error_response(error),
    };

    let metadata = match tokio::fs::metadata(&path).await {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return error_response(StatusCode::NOT_FOUND, "not found");
        }
        Err(error) => {
            eprintln!("server metadata error: {error}");
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "filesystem error");
        }
    };
    if !metadata.is_file() {
        return error_response(StatusCode::NOT_FOUND, "not found");
    }

    let etag = entity_tag(&metadata);
    let modified_header = last_modified(&metadata);
    let modified = metadata.modified().ok();
    let cache_control = if segments.first().map(String::as_str) == Some("thumbs") {
        "public, max-age=2592000, immutable"
    } else {
        "no-cache"
    };

    if is_not_modified(&headers, &etag, modified) {
        return not_modified_response(&etag, modified_header, cache_control);
    }

    let length = metadata.len();
    let content_type = content_type_for(&path);

    let body = if method == Method::HEAD {
        Body::empty()
    } else {
        match tokio::fs::File::open(&path).await {
            Ok(file) => Body::from_stream(ReaderStream::new(file)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return error_response(StatusCode::NOT_FOUND, "not found");
            }
            Err(error) => {
                eprintln!("server open error: {error}");
                return error_response(StatusCode::INTERNAL_SERVER_ERROR, "filesystem error");
            }
        }
    };

    let mut response = Response::new(body);
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, content_type);
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
    if let Some(value) = modified_header {
        headers.insert(header::LAST_MODIFIED, value);
    }
    response
}
