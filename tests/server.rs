use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use axum::body::Body;
use axum::http::{header, HeaderMap, HeaderName, HeaderValue, Method, Request, StatusCode};
use axum::Router;
use media_catalog::server::{app_state, router, router_from_state, serve_listener, ServerConfig};
use tempfile::TempDir;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tower::ServiceExt;

struct Fixture {
    _temp: TempDir,
    public: PathBuf,
    media: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let catalog = temp.path().join("catalog");
        let public = catalog.join("public");
        let media = temp.path().join("media");

        fs::create_dir_all(public.join("assets")).unwrap();
        fs::create_dir_all(public.join("catalog")).unwrap();
        fs::create_dir_all(public.join("2024/05")).unwrap();
        fs::create_dir_all(public.join("thumbs/2024/05")).unwrap();
        fs::create_dir_all(media.join("2024/05")).unwrap();

        fs::write(public.join("index.html"), "<h1>root</h1>").unwrap();
        fs::write(public.join("search.html"), "<h1>search</h1>").unwrap();
        fs::write(public.join("2024/index.html"), "<h1>2024</h1>").unwrap();
        fs::write(public.join("2024/05/index.html"), "<h1>May 2024</h1>").unwrap();
        fs::write(public.join("assets/site.css"), "body { color: black; }").unwrap();
        fs::write(public.join("catalog/index.json"), "{\"schema_version\":1}").unwrap();
        fs::write(
            public.join("catalog/2024.json"),
            "{\"year\":2024,\"items\":[]}",
        )
        .unwrap();
        fs::write(public.join("thumbs/2024/05/a.jpg.webp"), b"thumbnail-bytes").unwrap();
        fs::write(
            public.join("thumbs/2024/05/Familie Müller #1.jpg.webp"),
            b"unicode-thumbnail",
        )
        .unwrap();

        fs::write(media.join("2024/05/a.jpg"), [1u8, 2, 3, 4]).unwrap();
        fs::write(
            media.join("2024/05/Familie Müller #1.jpg"),
            b"unicode-media",
        )
        .unwrap();
        fs::write(media.join("2024/05/clip.mp4"), byte_pattern(100)).unwrap();
        fs::write(media.join("2024/05/large.mp4"), vec![0xAB; 4 * 1024 * 1024]).unwrap();
        fs::write(media.join("2024/05/notes.txt"), b"not-media").unwrap();

        // State files deliberately live outside the served public tree.
        fs::write(catalog.join("state.sqlite"), b"state").unwrap();
        fs::write(catalog.join("build.lock"), b"").unwrap();

        Self {
            _temp: temp,
            public,
            media,
        }
    }

    fn config(&self) -> ServerConfig {
        ServerConfig::new(&self.public, &self.media)
    }
}

fn byte_pattern(size: usize) -> Vec<u8> {
    (0..size).map(|index| (index % 251) as u8).collect()
}

async fn send_with(
    app: Router,
    method: Method,
    uri: &str,
    headers: Vec<(HeaderName, HeaderValue)>,
) -> (StatusCode, HeaderMap, Vec<u8>) {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .body(Body::empty())
        .unwrap();
    for (name, value) in headers {
        request.headers_mut().insert(name, value);
    }
    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap()
        .to_vec();
    (status, headers, body)
}

async fn send(app: Router, method: Method, uri: &str) -> (StatusCode, HeaderMap, Vec<u8>) {
    send_with(app, method, uri, Vec::new()).await
}

fn header_value(headers: &HeaderMap, name: HeaderName) -> &str {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .unwrap()
}

#[tokio::test]
async fn serves_generated_catalog_and_hides_state() {
    let fixture = Fixture::new();
    let app = router(fixture.config()).unwrap();

    let (status, headers, body) = send(app.clone(), Method::GET, "/").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, b"<h1>root</h1>");
    assert!(header_value(&headers, header::CONTENT_TYPE).starts_with("text/html"));

    let (status, _, body) = send(app.clone(), Method::GET, "/2024/").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, b"<h1>2024</h1>");

    let (status, _, body) = send(app.clone(), Method::GET, "/2024/05/").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, b"<h1>May 2024</h1>");

    let (status, _, body) = send(app.clone(), Method::GET, "/2024").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, b"<h1>2024</h1>");

    let (status, headers, _) = send(app.clone(), Method::GET, "/assets/site.css").await;
    assert_eq!(status, StatusCode::OK);
    assert!(header_value(&headers, header::CONTENT_TYPE).starts_with("text/css"));

    let (status, headers, _) = send(app.clone(), Method::GET, "/catalog/index.json").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        header_value(&headers, header::CONTENT_TYPE),
        "application/json"
    );
    assert_eq!(header_value(&headers, header::CACHE_CONTROL), "no-cache");

    let (status, headers, _) =
        send(app.clone(), Method::GET, "/thumbs/2024/05/a.jpg.webp?v=1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(header_value(&headers, header::CONTENT_TYPE), "image/webp");
    assert_eq!(
        header_value(&headers, header::CACHE_CONTROL),
        "public, max-age=2592000, immutable"
    );

    for path in [
        "/state.sqlite",
        "/build.lock",
        "/missing",
        "/thumbs/",
        "/2024/05/unknown",
    ] {
        let (status, _, _) = send(app.clone(), Method::GET, path).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{path}");
    }
}

#[tokio::test]
async fn serves_unicode_and_encoded_paths() {
    let fixture = Fixture::new();
    let app = router(fixture.config()).unwrap();

    let (status, headers, body) = send(
        app.clone(),
        Method::GET,
        "/media/2024/05/Familie%20M%C3%BCller%20%231.jpg",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, b"unicode-media");
    assert_eq!(header_value(&headers, header::CONTENT_TYPE), "image/jpeg");

    let (status, headers, body) = send(
        app.clone(),
        Method::GET,
        "/thumbs/2024/05/Familie%20M%C3%BCller%20%231.jpg.webp?v=1",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, b"unicode-thumbnail");
    assert_eq!(header_value(&headers, header::CONTENT_TYPE), "image/webp");
}

#[tokio::test]
async fn streams_media_with_head_and_ranges() {
    let fixture = Fixture::new();
    let app = router(fixture.config()).unwrap();
    let expected = byte_pattern(100);

    let (status, headers, body) = send(app.clone(), Method::GET, "/media/2024/05/clip.mp4").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, expected);
    assert_eq!(header_value(&headers, header::CONTENT_TYPE), "video/mp4");
    assert_eq!(header_value(&headers, header::ACCEPT_RANGES), "bytes");
    assert_eq!(header_value(&headers, header::CONTENT_LENGTH), "100");
    assert_eq!(
        header_value(&headers, HeaderName::from_static("x-content-type-options")),
        "nosniff"
    );

    let (status, headers, body) = send(app.clone(), Method::HEAD, "/media/2024/05/clip.mp4").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.is_empty());
    assert_eq!(header_value(&headers, header::CONTENT_LENGTH), "100");

    let (status, headers, body) = send_with(
        app.clone(),
        Method::GET,
        "/media/2024/05/clip.mp4",
        vec![(header::RANGE, HeaderValue::from_static("bytes=10-19"))],
    )
    .await;
    assert_eq!(status, StatusCode::PARTIAL_CONTENT);
    assert_eq!(body, expected[10..20]);
    assert_eq!(
        header_value(&headers, header::CONTENT_RANGE),
        "bytes 10-19/100"
    );
    assert_eq!(header_value(&headers, header::CONTENT_LENGTH), "10");

    let (status, _, body) = send_with(
        app.clone(),
        Method::GET,
        "/media/2024/05/clip.mp4",
        vec![(header::RANGE, HeaderValue::from_static("bytes=90-"))],
    )
    .await;
    assert_eq!(status, StatusCode::PARTIAL_CONTENT);
    assert_eq!(body, expected[90..]);

    let (status, _, body) = send_with(
        app.clone(),
        Method::GET,
        "/media/2024/05/clip.mp4",
        vec![(header::RANGE, HeaderValue::from_static("bytes=-5"))],
    )
    .await;
    assert_eq!(status, StatusCode::PARTIAL_CONTENT);
    assert_eq!(body, expected[95..]);

    let (status, headers, _) = send_with(
        app.clone(),
        Method::GET,
        "/media/2024/05/clip.mp4",
        vec![(header::RANGE, HeaderValue::from_static("bytes=200-300"))],
    )
    .await;
    assert_eq!(status, StatusCode::RANGE_NOT_SATISFIABLE);
    assert_eq!(header_value(&headers, header::CONTENT_RANGE), "bytes */100");
    assert_eq!(
        header_value(&headers, header::CACHE_CONTROL),
        "private, no-cache"
    );

    let (status, _, _) = send(app.clone(), Method::GET, "/media/2024/05/missing.jpg").await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, _, _) = send(app.clone(), Method::GET, "/media/2024/05/notes.txt").await;
    assert_eq!(status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
}

#[tokio::test]
async fn rejects_unsafe_paths_and_symlink_escape() {
    let fixture = Fixture::new();
    let app = router(fixture.config()).unwrap();

    for path in [
        "/media/../etc/passwd",
        "/media/%2e%2e%2fetc/passwd",
        "/media/2024/05/a%00.jpg",
        "/media/2024/05/a%5Cb.jpg",
    ] {
        let (status, _, _) = send(app.clone(), Method::GET, path).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{path}");
    }

    #[cfg(unix)]
    {
        let outside = fixture._temp.path().join("outside.jpg");
        fs::write(&outside, b"outside").unwrap();
        std::os::unix::fs::symlink(&outside, fixture.media.join("2024/05/escape.jpg")).unwrap();
        let (status, _, _) = send(app.clone(), Method::GET, "/media/2024/05/escape.jpg").await;
        assert_eq!(status, StatusCode::FORBIDDEN);
    }
}

#[tokio::test]
async fn reports_mount_health_and_recovers() {
    let fixture = Fixture::new();
    let app = router(fixture.config()).unwrap();
    let backup = fixture._temp.path().join("media-backup");

    fs::rename(&fixture.media, &backup).unwrap();
    let (status, headers, _) = send(app.clone(), Method::GET, "/media/2024/05/a.jpg").await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(header_value(&headers, header::CACHE_CONTROL), "no-store");
    let (status, headers, _) = send(app.clone(), Method::GET, "/healthz").await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(header_value(&headers, header::CACHE_CONTROL), "no-store");

    fs::rename(&backup, &fixture.media).unwrap();
    let (status, _, _) = send(app.clone(), Method::GET, "/media/2024/05/a.jpg").await;
    assert_eq!(status, StatusCode::OK);
    let (status, _, _) = send(app.clone(), Method::GET, "/healthz").await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn honors_conditional_requests_and_cache_policies() {
    let fixture = Fixture::new();
    let app = router(fixture.config()).unwrap();

    let (status, headers, _) = send(app.clone(), Method::GET, "/catalog/index.json").await;
    assert_eq!(status, StatusCode::OK);
    let etag = header_value(&headers, header::ETAG).to_string();
    let last_modified = header_value(&headers, header::LAST_MODIFIED).to_string();

    let (status, _, body) = send_with(
        app.clone(),
        Method::GET,
        "/catalog/index.json",
        vec![(header::IF_NONE_MATCH, HeaderValue::from_str(&etag).unwrap())],
    )
    .await;
    assert_eq!(status, StatusCode::NOT_MODIFIED);
    assert!(body.is_empty());

    let (status, _, _) = send_with(
        app.clone(),
        Method::GET,
        "/catalog/index.json",
        vec![(
            header::IF_MODIFIED_SINCE,
            HeaderValue::from_str(&last_modified).unwrap(),
        )],
    )
    .await;
    assert_eq!(status, StatusCode::NOT_MODIFIED);

    let (_, headers, _) = send(app.clone(), Method::GET, "/media/2024/05/clip.mp4").await;
    let etag = header_value(&headers, header::ETAG).to_string();
    let (status, _, body) = send_with(
        app.clone(),
        Method::GET,
        "/media/2024/05/clip.mp4",
        vec![(header::IF_NONE_MATCH, HeaderValue::from_str(&etag).unwrap())],
    )
    .await;
    assert_eq!(status, StatusCode::NOT_MODIFIED);
    assert!(body.is_empty());

    let (_, headers, _) = send(app.clone(), Method::GET, "/missing").await;
    assert_eq!(header_value(&headers, header::CACHE_CONTROL), "no-store");
}

#[tokio::test]
async fn enforces_access_token() {
    let fixture = Fixture::new();
    let mut config = fixture.config();
    config.token = Some("secret".to_string());
    let app = router(config).unwrap();

    let (status, headers, _) = send(app.clone(), Method::GET, "/").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(header_value(&headers, header::WWW_AUTHENTICATE), "Bearer");

    let (status, _, _) = send_with(
        app.clone(),
        Method::GET,
        "/",
        vec![(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer wrong"),
        )],
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, _, body) = send_with(
        app.clone(),
        Method::GET,
        "/",
        vec![(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer secret"),
        )],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, b"<h1>root</h1>");

    let (status, _, _) = send(app.clone(), Method::GET, "/healthz").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn bounds_concurrent_media_streams() {
    let fixture = Fixture::new();
    let mut config = fixture.config();
    config.max_streams = 1;
    let state = app_state(config).unwrap();
    assert_eq!(state.available_streams(), 1);
    let held = state.try_acquire_stream().unwrap();
    let app = router_from_state(state);

    let (status, _, _) = send(app.clone(), Method::GET, "/media/2024/05/a.jpg").await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);

    drop(held);
    let (status, _, _) = send(app.clone(), Method::GET, "/media/2024/05/a.jpg").await;
    assert_eq!(status, StatusCode::OK);
}

async fn read_head(stream: &mut TcpStream) -> (String, Vec<u8>) {
    let mut buffer = Vec::new();
    loop {
        let mut chunk = [0u8; 4096];
        let read = stream.read(&mut chunk).await.unwrap();
        assert!(read > 0, "connection closed before headers");
        buffer.extend_from_slice(&chunk[..read]);
        if let Some(position) = buffer.windows(4).position(|window| window == b"\r\n\r\n") {
            let head = String::from_utf8_lossy(&buffer[..position]).to_string();
            let rest = buffer[position + 4..].to_vec();
            return (head, rest);
        }
    }
}

fn content_length(head: &str) -> usize {
    head.lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            if name.eq_ignore_ascii_case("content-length") {
                value.trim().parse().ok()
            } else {
                None
            }
        })
        .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graceful_shutdown_drains_and_releases_port() {
    let fixture = Fixture::new();
    let config = fixture.config();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        serve_listener(listener, config, async move {
            let _ = shutdown_rx.await;
        })
        .await
    });

    let mut stream = TcpStream::connect(address).await.unwrap();
    stream
        .write_all(
            b"GET /media/2024/05/large.mp4 HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
        )
        .await
        .unwrap();
    let (head, rest) = read_head(&mut stream).await;
    assert!(head.starts_with("HTTP/1.1 200"), "{head}");
    let length = content_length(&head);
    assert_eq!(length, 4 * 1024 * 1024);

    shutdown_tx.send(()).unwrap();

    let mut body = rest;
    while body.len() < length {
        let mut chunk = vec![0u8; length - body.len()];
        let read = stream.read(&mut chunk).await.unwrap();
        assert!(read > 0, "response ended before the full body");
        body.extend_from_slice(&chunk[..read]);
    }
    assert_eq!(body.len(), length);
    drop(stream);

    let result = tokio::time::timeout(Duration::from_secs(10), server)
        .await
        .expect("server did not shut down")
        .expect("server task panicked");
    result.unwrap();

    let rebound = TcpListener::bind(address).await;
    assert!(rebound.is_ok(), "listener port was not released");
}

#[tokio::test]
async fn startup_validation_rejects_missing_media_root() {
    let fixture = Fixture::new();
    let missing = fixture._temp.path().join("missing-media");
    let config = ServerConfig::new(&fixture.public, missing);
    assert!(router(config).is_err());
}

#[tokio::test]
async fn startup_validation_rejects_missing_public_root() {
    let fixture = Fixture::new();
    let missing = fixture._temp.path().join("missing-public");
    let config = ServerConfig::new(missing, &fixture.media);
    assert!(router(config).is_err());
}
