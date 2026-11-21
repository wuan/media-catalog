use axum::body::Body;
use axum::extract::State;
use axum::http::{header, HeaderMap, HeaderValue, Method, StatusCode, Uri};
use axum::response::Response;
use tokio::io::{AsyncReadExt, AsyncSeekExt, SeekFrom};

use super::support::{
    decode_segments, entity_tag, error_response, is_not_modified, last_modified,
    media_content_type, method_not_allowed, not_modified_response, parse_range,
    resolve_error_response, resolve_file, RangeRequest,
};
use super::AppState;

/// Serve a read-only original from the media root.
pub async fn serve_media(
    State(state): State<AppState>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
) -> Response {
    if !matches!(method, Method::GET | Method::HEAD) {
        return method_not_allowed();
    }

    if !state.media_root.is_dir() {
        return error_response(StatusCode::SERVICE_UNAVAILABLE, "media root unavailable");
    }

    let Some(raw) = uri.path().strip_prefix("/media/") else {
        return error_response(StatusCode::NOT_FOUND, "not found");
    };
    if raw.is_empty() || raw.ends_with('/') {
        return error_response(StatusCode::NOT_FOUND, "not found");
    }

    let decoded = match decode_segments(raw, false) {
        Ok(decoded) => decoded,
        Err(()) => return error_response(StatusCode::BAD_REQUEST, "invalid path"),
    };
    if decoded.parts.is_empty() {
        return error_response(StatusCode::NOT_FOUND, "not found");
    }

    let path = match resolve_file(&state.media_root, &decoded.parts).await {
        Ok(path) => path,
        Err(error) => return resolve_error_response(error),
    };

    let metadata = match tokio::fs::metadata(&path).await {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return error_response(StatusCode::NOT_FOUND, "not found");
        }
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            return error_response(StatusCode::FORBIDDEN, "forbidden");
        }
        Err(error) => {
            eprintln!("server metadata error: {error}");
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "filesystem error");
        }
    };
    if !metadata.is_file() {
        return error_response(StatusCode::NOT_FOUND, "not found");
    }

    let Some(content_type) = media_content_type(&path) else {
        return error_response(StatusCode::UNSUPPORTED_MEDIA_TYPE, "unsupported media type");
    };

    let etag = entity_tag(&metadata);
    let modified_header = last_modified(&metadata);
    let modified = metadata.modified().ok();
    let cache_control = "private, no-cache";

    if is_not_modified(&headers, &etag, modified) {
        return not_modified_response(&etag, modified_header, cache_control);
    }

    let size = metadata.len();
    let requested_range = if method == Method::GET {
        headers
            .get(header::RANGE)
            .and_then(|value| value.to_str().ok())
            .map(|value| parse_range(value, size))
            .unwrap_or(RangeRequest::None)
    } else {
        RangeRequest::None
    };

    if requested_range == RangeRequest::Unsatisfiable {
        return range_not_satisfiable(size, &etag, modified_header);
    }

    let (status, start, length, content_range) = match requested_range {
        RangeRequest::Satisfiable { start, end } => (
            StatusCode::PARTIAL_CONTENT,
            start,
            end - start + 1,
            Some(format!("bytes {start}-{end}/{size}")),
        ),
        RangeRequest::None | RangeRequest::Unsatisfiable => (StatusCode::OK, 0, size, None),
    };

    if method == Method::HEAD {
        return file_response(
            status,
            Body::empty(),
            content_type,
            length,
            content_range,
            &etag,
            modified_header,
            cache_control,
        );
    }

    let permit = match state.media_streams.clone().try_acquire_owned() {
        Ok(permit) => permit,
        Err(_) => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "too many concurrent media streams",
            );
        }
    };

    let mut file = match tokio::fs::File::open(&path).await {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return error_response(StatusCode::NOT_FOUND, "not found");
        }
        Err(error) => {
            eprintln!("server open error: {error}");
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "filesystem error");
        }
    };

    if start > 0 {
        if let Err(error) = file.seek(SeekFrom::Start(start)).await {
            eprintln!("server seek error: {error}");
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "filesystem error");
        }
    }

    file_response(
        status,
        Body::from_stream(media_stream(file, length, permit)),
        content_type,
        length,
        content_range,
        &etag,
        modified_header,
        cache_control,
    )
}

fn media_stream(
    mut file: tokio::fs::File,
    length: u64,
    permit: tokio::sync::OwnedSemaphorePermit,
) -> impl futures_core::Stream<Item = Result<axum::body::Bytes, std::io::Error>> {
    async_stream::try_stream! {
        let _permit = permit;
        let mut remaining = length;
        let mut buffer = vec![0u8; 64 * 1024];
        while remaining > 0 {
            let read_len = std::cmp::min(buffer.len() as u64, remaining) as usize;
            let read = file.read(&mut buffer[..read_len]).await?;
            if read == 0 {
                break;
            }
            remaining -= read as u64;
            yield axum::body::Bytes::copy_from_slice(&buffer[..read]);
        }
    }
}

fn range_not_satisfiable(size: u64, etag: &str, modified: Option<HeaderValue>) -> Response {
    let mut response = error_response(StatusCode::RANGE_NOT_SATISFIABLE, "range not satisfiable");
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_RANGE,
        HeaderValue::from_str(&format!("bytes */{size}"))
            .unwrap_or_else(|_| HeaderValue::from_static("bytes */0")),
    );
    headers.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    if let Ok(value) = HeaderValue::from_str(etag) {
        headers.insert(header::ETAG, value);
    }
    if let Some(value) = modified {
        headers.insert(header::LAST_MODIFIED, value);
    }
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, no-cache"),
    );
    response
}

#[allow(clippy::too_many_arguments)]
fn file_response(
    status: StatusCode,
    body: Body,
    content_type: &'static str,
    length: u64,
    content_range: Option<String>,
    etag: &str,
    modified: Option<HeaderValue>,
    cache_control: &'static str,
) -> Response {
    let mut response = Response::new(body);
    *response.status_mut() = status;
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    headers.insert(
        header::CONTENT_LENGTH,
        HeaderValue::from_str(&length.to_string())
            .unwrap_or_else(|_| HeaderValue::from_static("0")),
    );
    headers.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("inline"),
    );
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(cache_control),
    );
    if let Ok(value) = HeaderValue::from_str(etag) {
        headers.insert(header::ETAG, value);
    }
    if let Some(value) = modified {
        headers.insert(header::LAST_MODIFIED, value);
    }
    if let Some(value) = content_range {
        if let Ok(value) = HeaderValue::from_str(&value) {
            headers.insert(header::CONTENT_RANGE, value);
        }
    }
    response
}
