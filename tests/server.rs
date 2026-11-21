use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use axum::body::Body;
use axum::http::{header, HeaderMap, HeaderName, HeaderValue, Method, Request, StatusCode};
use axum::Router;
use media_catalog::json;
use media_catalog::model::{build_catalog, Catalog, CatalogItemInput};
use media_catalog::scan::MediaKind;
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

        fs::create_dir_all(public.join("catalog")).unwrap();
        fs::create_dir_all(public.join("thumbs/2024/05")).unwrap();
        fs::create_dir_all(media.join("2024/05")).unwrap();

        write_catalog(&public, &fixture_catalog());

        // Obsolete generated files may remain after an upgrade; the server must
        // ignore them and serve rendered pages and embedded assets instead.
        fs::create_dir_all(public.join("assets")).unwrap();
        fs::write(public.join("index.html"), "<h1>obsolete root file</h1>").unwrap();
        fs::write(public.join("search.html"), "<h1>obsolete search file</h1>").unwrap();
        fs::write(public.join("assets/site.css"), "obsolete css").unwrap();
        fs::write(public.join("assets/search.js"), "obsolete js").unwrap();

        fs::write(public.join("thumbs/2024/05/a.jpg.webp"), b"thumbnail-bytes").unwrap();
        fs::write(
            public.join("thumbs/2024/05/Familie Müller #1.jpg.webp"),
            b"unicode-thumbnail",
        )
        .unwrap();
        fs::write(
            public.join("thumbs/2024/05/clip.mp4.webp"),
            b"video-thumbnail",
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

fn fixture_catalog() -> Catalog {
    item_catalog(vec![
        item("2024/05/a.jpg", MediaKind::Image),
        item("2024/05/Familie Müller #1.jpg", MediaKind::Image),
        Catalogs::video("2024/05/clip.mp4"),
    ])
}

struct Catalogs;

impl Catalogs {
    fn video(path: &str) -> CatalogItemInput {
        CatalogItemInput {
            duration_ms: Some(65_000),
            kind: MediaKind::Video,
            ..item(path, MediaKind::Video)
        }
    }
}

fn item(path: &str, kind: MediaKind) -> CatalogItemInput {
    CatalogItemInput {
        rel_path: path.to_string(),
        year: 2024,
        month: 5,
        kind,
        captured_at: Some("2024-05-01 10:00:00".to_string()),
        mtime_ns: 0,
        width: Some(4000),
        height: Some(3000),
        duration_ms: None,
        thumb_version: Some("1".to_string()),
        thumb_available: true,
    }
}

fn item_catalog(items: Vec<CatalogItemInput>) -> Catalog {
    build_catalog(items, "/media", "/thumbs")
}

fn write_catalog(public: &Path, catalog: &Catalog) {
    let output = json::render_json(catalog).unwrap();
    let catalog_dir = public.join("catalog");
    fs::create_dir_all(&catalog_dir).unwrap();
    for (year, shard) in &output.shards {
        fs::write(catalog_dir.join(format!("{year}.json")), shard).unwrap();
    }
    fs::write(catalog_dir.join("index.json"), output.index).unwrap();
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
        .unwrap_or("<missing>")
}

#[tokio::test]
async fn renders_catalog_pages_and_hides_state() {
    let fixture = Fixture::new();
    let app = router(fixture.config()).unwrap();

    let (status, headers, body) = send(app.clone(), Method::GET, "/").await;
    assert_eq!(status, StatusCode::OK);
    assert!(header_value(&headers, header::CONTENT_TYPE).starts_with("text/html"));
    let html = String::from_utf8_lossy(&body);
    assert!(html.contains("2024"));
    assert!(html.contains("Years"));
    assert!(!html.contains("obsolete root file"));

    let (status, _, body) = send(app.clone(), Method::GET, "/2024/").await;
    assert_eq!(status, StatusCode::OK);
    let html = String::from_utf8_lossy(&body);
    assert!(html.contains("May"));
    assert!(html.contains("2024"));

    let (status, _, body) = send(app.clone(), Method::GET, "/2024/05/").await;
    assert_eq!(status, StatusCode::OK);
    let html = String::from_utf8_lossy(&body);
    assert!(html.contains("a.jpg"));
    assert!(html.contains("href=\"/media/2024/05/a.jpg\""));
    assert!(html.contains("loading=\"lazy\""));
    assert!(html.contains("Video"));
    assert!(html.contains("1:05"));

    let (status, _, body) = send(app.clone(), Method::GET, "/2024").await;
    assert_eq!(status, StatusCode::OK);
    assert!(String::from_utf8_lossy(&body).contains("May"));

    let (status, headers, body) = send(app.clone(), Method::GET, "/assets/site.css").await;
    assert_eq!(status, StatusCode::OK);
    assert!(header_value(&headers, header::CONTENT_TYPE).starts_with("text/css"));
    let css = String::from_utf8_lossy(&body);
    assert!(css.contains(".site-header"));
    assert!(!css.contains("obsolete css"));

    let (status, headers, body) = send(app.clone(), Method::GET, "/assets/search.js").await;
    assert_eq!(status, StatusCode::OK);
    assert!(header_value(&headers, header::CONTENT_TYPE).starts_with("text/javascript"));
    assert!(String::from_utf8_lossy(&body).contains("__mediaCatalogSearch"));

    let (status, headers, body) = send(app.clone(), Method::GET, "/search.html").await;
    assert_eq!(status, StatusCode::OK);
    assert!(header_value(&headers, header::CONTENT_TYPE).starts_with("text/html"));
    let shell = String::from_utf8_lossy(&body);
    assert!(shell.contains("id=\"filters\""));
    assert!(!shell.contains("obsolete search file"));

    let (status, headers, body) = send(app.clone(), Method::GET, "/catalog/index.json").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        header_value(&headers, header::CONTENT_TYPE),
        "application/json"
    );
    assert_eq!(header_value(&headers, header::CACHE_CONTROL), "no-cache");
    let index: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(index["schema_version"], 1);
    assert_eq!(index["years"][0]["year"], 2024);

    let (status, headers, body) =
        send(app.clone(), Method::GET, "/thumbs/2024/05/a.jpg.webp?v=1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(header_value(&headers, header::CONTENT_TYPE), "image/webp");
    assert_eq!(
        header_value(&headers, header::CACHE_CONTROL),
        "public, max-age=2592000, immutable"
    );
    assert_eq!(body, b"thumbnail-bytes");

    for path in [
        "/state.sqlite",
        "/build.lock",
        "/missing",
        "/thumbs/",
        "/2024/05/unknown",
        "/2023/",
        "/2024/06/",
        "/2024/5/",
        "/abcd/",
        "/index.html",
        "/2024/index.html",
        "/2024/05/index.html",
        "/assets/other.css",
    ] {
        let (status, _, _) = send(app.clone(), Method::GET, path).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{path}");
    }
}

#[tokio::test]
async fn escapes_hostile_names_and_links_originals() {
    let fixture = Fixture::new();
    let hostile = item_catalog(vec![item("2024/05/<script>alert(1).jpg", MediaKind::Image)]);
    write_catalog(&fixture.public, &hostile);
    let app = router(fixture.config()).unwrap();

    let (status, _, body) = send(app.clone(), Method::GET, "/2024/05/").await;
    assert_eq!(status, StatusCode::OK);
    let html = String::from_utf8_lossy(&body);
    assert!(!html.contains("<script>alert(1)"));
    assert!(html.contains("href=\"/media/2024/05/%3Cscript%3Ealert(1).jpg\""));
    assert!(
        html.contains("alt=\"&#60;script&#62;alert(1).jpg\"")
            || html.contains("alt=\"&lt;script&gt;alert(1).jpg\"")
    );
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
async fn reports_separate_catalog_and_media_health_and_recovers() {
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

    // Catalog pages do not depend on the media mount.
    let (status, _, _) = send(app.clone(), Method::GET, "/").await;
    assert_eq!(status, StatusCode::OK);

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

    let (status, headers, _) = send(app.clone(), Method::GET, "/2024/05/").await;
    assert_eq!(status, StatusCode::OK);
    let page_etag = header_value(&headers, header::ETAG).to_string();
    assert_eq!(header_value(&headers, header::CACHE_CONTROL), "no-cache");

    let (status, _, body) = send_with(
        app.clone(),
        Method::GET,
        "/2024/05/",
        vec![(
            header::IF_NONE_MATCH,
            HeaderValue::from_str(&page_etag).unwrap(),
        )],
    )
    .await;
    assert_eq!(status, StatusCode::NOT_MODIFIED);
    assert!(body.is_empty());

    let (status, headers, body) = send(app.clone(), Method::HEAD, "/2024/05/").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.is_empty());
    assert!(
        header_value(&headers, header::CONTENT_LENGTH)
            .parse::<usize>()
            .unwrap()
            > 0
    );

    let (_, headers, _) = send(app.clone(), Method::GET, "/assets/site.css").await;
    let asset_etag = header_value(&headers, header::ETAG).to_string();
    let (status, _, _) = send_with(
        app.clone(),
        Method::GET,
        "/assets/site.css",
        vec![(
            header::IF_NONE_MATCH,
            HeaderValue::from_str(&asset_etag).unwrap(),
        )],
    )
    .await;
    assert_eq!(status, StatusCode::NOT_MODIFIED);

    let (status, headers, body) = send(app.clone(), Method::HEAD, "/assets/search.js").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.is_empty());
    assert!(
        header_value(&headers, header::CONTENT_LENGTH)
            .parse::<usize>()
            .unwrap()
            > 0
    );

    let (_, headers, _) = send(app.clone(), Method::GET, "/search.html").await;
    let shell_etag = header_value(&headers, header::ETAG).to_string();
    let (status, _, body) = send_with(
        app.clone(),
        Method::GET,
        "/search.html",
        vec![(
            header::IF_NONE_MATCH,
            HeaderValue::from_str(&shell_etag).unwrap(),
        )],
    )
    .await;
    assert_eq!(status, StatusCode::NOT_MODIFIED);
    assert!(body.is_empty());

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
    assert!(String::from_utf8_lossy(&body).contains("Years"));

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
async fn startup_validation_still_requires_the_media_root() {
    let fixture = Fixture::new();
    let missing = fixture._temp.path().join("missing-media");
    let config = ServerConfig::new(&fixture.public, missing);
    assert!(router(config).is_err());
}

#[tokio::test]
async fn starts_without_a_catalog_and_recovers_when_one_appears() {
    let fixture = Fixture::new();
    let late_public = fixture._temp.path().join("late-public");
    let config = ServerConfig::new(&late_public, &fixture.media);
    let app = router(config).unwrap();

    let (status, headers, body) = send(app.clone(), Method::GET, "/").await;
    assert_eq!(status, StatusCode::OK);
    assert!(header_value(&headers, header::CONTENT_TYPE).starts_with("text/html"));
    let html = String::from_utf8_lossy(&body);
    assert!(html.contains("not available yet"));
    assert_eq!(header_value(&headers, header::CACHE_CONTROL), "no-store");

    let (status, _, body) = send(app.clone(), Method::GET, "/healthz").await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(String::from_utf8_lossy(&body).contains("catalog unavailable"));

    for path in ["/search.html", "/assets/site.css", "/assets/search.js"] {
        let (status, _, _) = send(app.clone(), Method::GET, path).await;
        assert_eq!(status, StatusCode::OK, "{path}");
    }

    let (_, _, body) = send(app.clone(), Method::GET, "/assets/search.js").await;
    assert!(String::from_utf8_lossy(&body).contains("catalog is not available yet"));

    let (status, _, _) = send(app.clone(), Method::GET, "/catalog/index.json").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _, _) = send(app.clone(), Method::GET, "/2024/").await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);

    write_catalog(&late_public, &fixture_catalog());
    let (status, _, body) = send(app.clone(), Method::GET, "/").await;
    assert_eq!(status, StatusCode::OK);
    assert!(String::from_utf8_lossy(&body).contains("2024"));
    let (status, _, _) = send(app.clone(), Method::GET, "/healthz").await;
    assert_eq!(status, StatusCode::OK);
}
