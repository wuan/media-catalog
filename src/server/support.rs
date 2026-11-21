use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::body::Body;
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::Response;
use percent_encoding::percent_decode_str;

/// Decoded relative path segments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segments {
    pub parts: Vec<String>,
    pub trailing_slash: bool,
}

/// Errors while resolving a request path under a root.
#[derive(Debug)]
pub enum ResolveError {
    NotFound,
    Forbidden,
    Io(io::Error),
}

/// Percent-decode a path once and reject unsafe segments.
pub fn decode_segments(raw: &str, allow_trailing_slash: bool) -> Result<Segments, ()> {
    let decoded = percent_decode_str(raw).decode_utf8().map_err(|_| ())?;
    let trailing_slash = decoded.ends_with('/');
    if trailing_slash && !allow_trailing_slash {
        return Err(());
    }

    let mut parts: Vec<String> = decoded.split('/').map(str::to_string).collect();
    if trailing_slash {
        parts.pop();
    }

    for part in &parts {
        if part.is_empty()
            || part == "."
            || part == ".."
            || part.contains('\\')
            || part.chars().any(char::is_control)
        {
            return Err(());
        }
    }

    Ok(Segments {
        parts,
        trailing_slash,
    })
}

/// Resolve a relative path under an already-canonicalized root.
pub async fn resolve_file(root: &Path, segments: &[String]) -> Result<PathBuf, ResolveError> {
    let candidate = segments
        .iter()
        .fold(root.to_path_buf(), |path, segment| path.join(segment));
    let canonical = tokio::fs::canonicalize(&candidate)
        .await
        .map_err(map_io_error)?;
    if !canonical.starts_with(root) {
        return Err(ResolveError::Forbidden);
    }
    Ok(canonical)
}

/// Resolve a public-tree path, mapping directories to their `index.html`.
pub async fn resolve_public(root: &Path, segments: &[String]) -> Result<PathBuf, ResolveError> {
    let path = resolve_file(root, segments).await?;
    let metadata = tokio::fs::metadata(&path).await.map_err(map_io_error)?;
    if metadata.is_dir() {
        let mut with_index = segments.to_vec();
        with_index.push("index.html".to_string());
        let path = resolve_file(root, &with_index).await?;
        let metadata = tokio::fs::metadata(&path).await.map_err(map_io_error)?;
        if metadata.is_file() {
            Ok(path)
        } else {
            Err(ResolveError::NotFound)
        }
    } else if metadata.is_file() {
        Ok(path)
    } else {
        Err(ResolveError::NotFound)
    }
}

fn map_io_error(error: io::Error) -> ResolveError {
    match error.kind() {
        io::ErrorKind::NotFound => ResolveError::NotFound,
        io::ErrorKind::PermissionDenied => ResolveError::Forbidden,
        _ => ResolveError::Io(error),
    }
}

pub fn resolve_error_response(error: ResolveError) -> Response {
    match error {
        ResolveError::NotFound => error_response(StatusCode::NOT_FOUND, "not found"),
        ResolveError::Forbidden => error_response(StatusCode::FORBIDDEN, "forbidden"),
        ResolveError::Io(error) => {
            eprintln!("server filesystem error: {error}");
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "filesystem error")
        }
    }
}

/// Build a plain-text response with an explicit cache policy.
pub fn text_response(status: StatusCode, body: &str, cache_control: &'static str) -> Response {
    let mut response = Response::new(Body::from(body.to_string()));
    *response.status_mut() = status;
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/plain; charset=utf-8"),
    );
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(cache_control),
    );
    response
}

/// Build a plain-text error response that will not be cached.
pub fn error_response(status: StatusCode, message: &str) -> Response {
    text_response(status, &format!("{message}\n"), "no-store")
}

pub fn method_not_allowed() -> Response {
    let mut response = error_response(StatusCode::METHOD_NOT_ALLOWED, "method not allowed");
    response
        .headers_mut()
        .insert(header::ALLOW, HeaderValue::from_static("GET, HEAD"));
    response
}

/// Build a strong entity tag from file size and modification time.
pub fn entity_tag(metadata: &std::fs::Metadata) -> String {
    let mtime = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("\"{:x}-{:x}\"", metadata.len(), mtime)
}

pub fn last_modified(metadata: &std::fs::Metadata) -> Option<HeaderValue> {
    metadata
        .modified()
        .ok()
        .and_then(|time| HeaderValue::from_str(&httpdate::fmt_http_date(time)).ok())
}

/// Evaluate `If-None-Match` first, then `If-Modified-Since`.
pub fn is_not_modified(
    request_headers: &HeaderMap,
    etag: &str,
    modified: Option<SystemTime>,
) -> bool {
    if let Some(value) = request_headers
        .get(header::IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
    {
        return value
            .split(',')
            .map(str::trim)
            .any(|candidate| candidate == "*" || candidate.trim_start_matches("W/") == etag);
    }

    if let (Some(value), Some(modified)) = (
        request_headers
            .get(header::IF_MODIFIED_SINCE)
            .and_then(|value| value.to_str().ok()),
        modified,
    ) {
        if let Ok(since) = httpdate::parse_http_date(value) {
            let modified_secs = modified
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_secs())
                .unwrap_or(0);
            let since_secs = since
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_secs())
                .unwrap_or(0);
            return modified_secs <= since_secs;
        }
    }

    false
}

pub fn not_modified_response(
    etag: &str,
    modified: Option<HeaderValue>,
    cache_control: &'static str,
) -> Response {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::NOT_MODIFIED;
    let headers = response.headers_mut();
    if let Ok(value) = HeaderValue::from_str(etag) {
        headers.insert(header::ETAG, value);
    }
    if let Some(value) = modified {
        headers.insert(header::LAST_MODIFIED, value);
    }
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(cache_control),
    );
    response
}

pub fn content_type_for(path: &Path) -> HeaderValue {
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    let mut value = mime.to_string();
    if value.starts_with("text/") && !value.contains("charset=") {
        value.push_str("; charset=utf-8");
    }
    HeaderValue::from_str(&value)
        .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream"))
}

/// Content types allowed for originals under the media root.
pub fn media_content_type(path: &Path) -> Option<&'static str> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)?;
    match extension.as_str() {
        "jpg" | "jpeg" => Some("image/jpeg"),
        "png" => Some("image/png"),
        "webp" => Some("image/webp"),
        "heic" => Some("image/heic"),
        "heif" => Some("image/heif"),
        "mp4" | "m4v" => Some("video/mp4"),
        "mov" => Some("video/quicktime"),
        "mkv" => Some("video/x-matroska"),
        "avi" => Some("video/x-msvideo"),
        "webm" => Some("video/webm"),
        "hevc" => Some("video/h265"),
        _ => None,
    }
}

/// A parsed single byte range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangeRequest {
    None,
    Satisfiable { start: u64, end: u64 },
    Unsatisfiable,
}

pub fn parse_range(value: &str, size: u64) -> RangeRequest {
    let Some(rest) = value.strip_prefix("bytes=") else {
        return RangeRequest::None;
    };
    if rest.contains(',') {
        return RangeRequest::Unsatisfiable;
    }
    let Some((start_text, end_text)) = rest.split_once('-') else {
        return RangeRequest::None;
    };

    if start_text.is_empty() {
        let Ok(suffix) = end_text.parse::<u64>() else {
            return RangeRequest::None;
        };
        if suffix == 0 || size == 0 {
            return RangeRequest::Unsatisfiable;
        }
        let start = size.saturating_sub(suffix);
        return RangeRequest::Satisfiable {
            start,
            end: size - 1,
        };
    }

    let Ok(start) = start_text.parse::<u64>() else {
        return RangeRequest::None;
    };
    if start >= size {
        return RangeRequest::Unsatisfiable;
    }
    let end = if end_text.is_empty() {
        size - 1
    } else {
        match end_text.parse::<u64>() {
            Ok(end) => end.min(size - 1),
            Err(_) => return RangeRequest::None,
        }
    };
    if end < start {
        return RangeRequest::Unsatisfiable;
    }
    RangeRequest::Satisfiable { start, end }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_traversal_and_control_characters() {
        assert!(decode_segments("../etc/passwd", false).is_err());
        assert!(decode_segments("%2e%2e%2fetc/passwd", false).is_err());
        assert!(decode_segments("a\\b", false).is_err());
        assert!(decode_segments("a%00b", false).is_err());
        assert!(decode_segments("a%0Ab", false).is_err());
    }

    #[test]
    fn allows_trailing_slash_only_when_requested() {
        let segments = decode_segments("2024/05/", true).unwrap();
        assert_eq!(segments.parts, vec!["2024", "05"]);
        assert!(segments.trailing_slash);
        assert!(decode_segments("2024/05/", false).is_err());
    }

    #[test]
    fn parses_ranges() {
        assert_eq!(
            parse_range("bytes=10-19", 100),
            RangeRequest::Satisfiable { start: 10, end: 19 }
        );
        assert_eq!(
            parse_range("bytes=90-", 100),
            RangeRequest::Satisfiable { start: 90, end: 99 }
        );
        assert_eq!(
            parse_range("bytes=-5", 100),
            RangeRequest::Satisfiable { start: 95, end: 99 }
        );
        assert_eq!(
            parse_range("bytes=200-300", 100),
            RangeRequest::Unsatisfiable
        );
        assert_eq!(parse_range("items=0-1", 100), RangeRequest::None);
    }

    #[test]
    fn maps_media_extensions() {
        assert_eq!(media_content_type(Path::new("a.jpg")), Some("image/jpeg"));
        assert_eq!(
            media_content_type(Path::new("a.MKV")),
            Some("video/x-matroska")
        );
        assert_eq!(media_content_type(Path::new("a.txt")), None);
    }
}
