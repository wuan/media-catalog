mod common;

use std::time::Duration;

use axum::body::Body;
use axum::http::{header, HeaderMap, HeaderName, HeaderValue, Method, Request, StatusCode};
use axum::Router;
use common::Fixture;
use media_catalog::run_with_config;
use media_catalog::server::{router, ServerConfig};
use serde_json::Value;
use tower::ServiceExt;

async fn send(
    app: &Router,
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
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap()
        .to_vec();
    (status, headers, body)
}

fn header_value(headers: &HeaderMap, name: HeaderName) -> String {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string()
}

fn html(body: &[u8]) -> String {
    String::from_utf8_lossy(body).to_string()
}

#[tokio::test]
async fn generator_output_is_served_end_to_end() {
    let fixture = Fixture::new();
    fixture.write_image("2024/05/alpha.jpg", 24, 18, 1);
    fixture.write_image("2023/12/gamma.jpg", 18, 24, 2);

    let outcome = run_with_config(fixture.config()).unwrap();
    assert!(
        outcome.summary.errors.is_empty(),
        "{:?}",
        outcome.summary.errors
    );

    let app = router(ServerConfig::new(&fixture.data, &fixture.media)).unwrap();

    let (status, headers, body) = send(&app, Method::GET, "/", Vec::new()).await;
    assert_eq!(status, StatusCode::OK);
    assert!(header_value(&headers, header::CONTENT_TYPE).starts_with("text/html"));
    let root = html(&body);
    assert!(root.contains("2024"));
    assert!(root.contains("2023"));
    assert!(root.contains("href=\"/2024/\""));

    let (status, _, body) = send(&app, Method::GET, "/2024/", Vec::new()).await;
    assert_eq!(status, StatusCode::OK);
    assert!(html(&body).contains("href=\"/2024/05/\""));

    let (status, _, body) = send(&app, Method::GET, "/2024/05/", Vec::new()).await;
    assert_eq!(status, StatusCode::OK);
    let month = html(&body);
    assert!(month.contains("alpha.jpg"));
    assert!(month.contains("href=\"/media/2024/05/alpha.jpg\""));
    assert!(month.contains("/thumbs/2024/05/alpha.jpg.webp?v="));

    let index: Value = serde_json::from_slice(
        &send(&app, Method::GET, "/catalog/index.json", Vec::new())
            .await
            .2,
    )
    .unwrap();
    assert_eq!(index["total_items"], 2);
    let shard_path = index["years"][0]["shard"].as_str().unwrap();
    let (status, headers, shard_body) =
        send(&app, Method::GET, &format!("/{shard_path}"), Vec::new()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        header_value(&headers, header::CONTENT_TYPE),
        "application/json"
    );
    let shard: Value = serde_json::from_slice(&shard_body).unwrap();
    let alpha = shard["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["path"] == "2024/05/alpha.jpg")
        .unwrap();
    let thumb_url = alpha["thumb_url"].as_str().unwrap();
    let media_url = alpha["media_url"].as_str().unwrap();

    let (status, headers, thumb_body) = send(&app, Method::GET, thumb_url, Vec::new()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(header_value(&headers, header::CONTENT_TYPE), "image/webp");
    assert!(!thumb_body.is_empty());

    let (status, headers, media_body) = send(&app, Method::GET, media_url, Vec::new()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(header_value(&headers, header::CONTENT_TYPE), "image/jpeg");
    assert_eq!(
        media_body,
        std::fs::read(fixture.media_path("2024/05/alpha.jpg")).unwrap()
    );

    let (status, _, shell) = send(&app, Method::GET, "/search.html", Vec::new()).await;
    assert_eq!(status, StatusCode::OK);
    assert!(html(&shell).contains("id=\"filters\""));
    let (status, _, script) = send(&app, Method::GET, "/assets/search.js", Vec::new()).await;
    assert_eq!(status, StatusCode::OK);
    assert!(html(&script).contains("__mediaCatalogSearch"));
}

#[tokio::test]
async fn running_server_picks_up_a_catalog_update_without_restart() {
    let fixture = Fixture::new();
    fixture.write_image("2024/05/alpha.jpg", 24, 18, 1);
    run_with_config(fixture.config()).unwrap();

    let app = router(ServerConfig::new(&fixture.data, &fixture.media)).unwrap();

    let (status, headers, body) = send(&app, Method::GET, "/2024/05/", Vec::new()).await;
    assert_eq!(status, StatusCode::OK);
    let first_etag = header_value(&headers, header::ETAG);
    assert!(!first_etag.is_empty());
    assert!(html(&body).contains("alpha.jpg"));

    std::thread::sleep(Duration::from_millis(20));
    fixture.write_image("2024/05/beta.jpg", 24, 18, 9);
    run_with_config(fixture.config()).unwrap();

    let (status, headers, body) = send(&app, Method::GET, "/2024/05/", Vec::new()).await;
    assert_eq!(status, StatusCode::OK);
    let second_etag = header_value(&headers, header::ETAG);
    assert_ne!(second_etag, first_etag);
    assert!(html(&body).contains("beta.jpg"));

    let (status, _, _) = send(
        &app,
        Method::GET,
        "/2024/05/",
        vec![(
            header::IF_NONE_MATCH,
            HeaderValue::from_str(&first_etag).unwrap(),
        )],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}
