mod common;

use std::time::Duration;

use axum::body::Body;
use axum::http::{header, HeaderMap, HeaderName, HeaderValue, Method, Request, StatusCode};
use axum::Router;
use common::Fixture;
use media_catalog::json;
use media_catalog::model::build_catalog;
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
    assert!(
        root.find("href=\"/2024/\"").unwrap() < root.find("href=\"/2023/\"").unwrap(),
        "root years must be newest first"
    );

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
    let years = index["years"]
        .as_array()
        .unwrap()
        .iter()
        .map(|year| year["year"].as_i64().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(years, vec![2024, 2023]);
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
async fn event_browsing_is_served_end_to_end() {
    let fixture = Fixture::new();
    fixture.write_jpeg_with_exif(
        "2024/05/alpha.jpg",
        Some(1),
        Some("2024:05:30 10:00:00"),
        false,
    );
    fixture.write_jpeg_with_exif(
        "2024/06/beta.jpg",
        Some(1),
        Some("2024:06:02 09:00:00"),
        false,
    );
    fixture.write_jpeg_with_exif(
        "2024/06/outside.jpg",
        Some(1),
        Some("2024:06:20 09:00:00"),
        false,
    );
    let events_path = fixture.root().join("events.toml");
    std::fs::write(
        &events_path,
        r#"
[[events]]
slug = "may-trip"
title = "May trip"
start = "2024-05-28 09:00"
end = "2024-06-05 18:30"
description = "A crossing event."
"#,
    )
    .unwrap();

    let mut config = fixture.config();
    config.events_file = Some(events_path);
    let outcome = run_with_config(config).unwrap();
    assert!(
        outcome.summary.errors.is_empty(),
        "{:?}",
        outcome.summary.errors
    );
    assert_eq!(outcome.summary.event_counts.len(), 1);
    assert_eq!(outcome.summary.event_counts[0].slug, "may-trip");
    assert_eq!(outcome.summary.event_counts[0].count, 2);
    assert!(outcome.summary.event_warnings.is_empty());

    let app = router(ServerConfig::new(&fixture.data, &fixture.media)).unwrap();

    let (status, _, body) = send(&app, Method::GET, "/events/", Vec::new()).await;
    assert_eq!(status, StatusCode::OK);
    let list = html(&body);
    assert!(list.contains("May trip"));
    assert!(list.contains("2 items"));
    assert!(list.contains("href=\"/events/may-trip/\""));

    let (status, _, body) = send(&app, Method::GET, "/events/may-trip/", Vec::new()).await;
    assert_eq!(status, StatusCode::OK);
    let event = html(&body);
    assert!(event.find("May 2024").unwrap() < event.find("June 2024").unwrap());
    assert!(event.contains("alpha.jpg"));
    assert!(event.contains("beta.jpg"));
    assert!(!event.contains("outside.jpg"));
    assert!(event.contains("href=\"/2024/06/\""));
    assert!(event.contains("href=\"/2024/05/\""));
    assert!(event.contains("loading=\"lazy\""));

    let (status, _, body) = send(&app, Method::GET, "/2024/05/", Vec::new()).await;
    assert_eq!(status, StatusCode::OK);
    let may = html(&body);
    assert!(may.contains("alpha.jpg"));
    assert!(!may.contains("beta.jpg"));

    let (status, _, body) = send(&app, Method::GET, "/2024/06/", Vec::new()).await;
    assert_eq!(status, StatusCode::OK);
    let june = html(&body);
    assert!(june.contains("beta.jpg"));
    assert!(june.contains("outside.jpg"));
    assert!(!june.contains("alpha.jpg"));

    let (status, _, body) = send(&app, Method::GET, "/", Vec::new()).await;
    assert_eq!(status, StatusCode::OK);
    let root = html(&body);
    assert!(root.find("Events").unwrap() < root.find("Years").unwrap());
    assert!(root.contains("href=\"/events/may-trip/\""));
}

#[tokio::test]
async fn root_page_shows_recent_items_newest_first() {
    let fixture = Fixture::new();
    fixture.write_jpeg_with_exif(
        "2023/12/old.jpg",
        Some(1),
        Some("2023:12:01 10:00:00"),
        false,
    );
    fixture.write_jpeg_with_exif(
        "2024/05/early.jpg",
        Some(1),
        Some("2024:05:10 10:00:00"),
        false,
    );
    fixture.write_jpeg_with_exif(
        "2024/06/newest.jpg",
        Some(1),
        Some("2024:06:20 10:00:00"),
        false,
    );
    if !fixture.write_video("2024/05/clip.mp4", 2) {
        eprintln!("skipping root recent item test: ffmpeg unavailable");
        return;
    }
    // Capture timestamps can disagree with folder position. Put the video
    // between the newest image and the May image so recency is global.
    let video_path = fixture.media_path("2024/05/clip.mp4");
    let touched = std::process::Command::new("touch")
        .arg("-t")
        .arg("202406100900")
        .arg(&video_path)
        .status()
        .expect("set video modification time");
    assert!(touched.success());

    let outcome = run_with_config(fixture.config()).unwrap();
    assert!(
        outcome.summary.errors.is_empty(),
        "{:?}",
        outcome.summary.errors
    );

    let app = router(ServerConfig::new(&fixture.data, &fixture.media)).unwrap();

    let (status, _, body) = send(&app, Method::GET, "/", Vec::new()).await;
    assert_eq!(status, StatusCode::OK);
    let root = html(&body);
    let years_heading = root.find("<h1>Years</h1>").expect("years heading");
    let recent_heading = root.find("<h1>Recent</h1>").expect("recent heading");
    assert!(
        years_heading < recent_heading,
        "the recent section must follow Years"
    );

    let recent = &root[recent_heading..];
    let newest = recent
        .find("href=\"/media/2024/06/newest.jpg\"")
        .expect("newest image");
    let video = recent
        .find("href=\"/media/2024/05/clip.mp4\"")
        .expect("recent video");
    let early = recent
        .find("href=\"/media/2024/05/early.jpg\"")
        .expect("May image");
    let old = recent
        .find("href=\"/media/2023/12/old.jpg\"")
        .expect("old image");
    assert!(
        newest < video && video < early && early < old,
        "recent tiles must be newest first across folders: {recent}"
    );
    assert!(recent.contains("data-kind=\"video\""));
    assert!(recent.contains("Video"));
    assert!(recent.contains("0:02"));
    assert_eq!(root.matches("/assets/viewer.js").count(), 1);
}

#[tokio::test]
async fn empty_published_catalog_omits_the_recent_section() {
    let fixture = Fixture::new();
    let catalog = build_catalog(vec![], "/media", "/thumbs");
    let output = json::render_json(&catalog).unwrap();
    let catalog_dir = fixture.data.join("catalog");
    std::fs::create_dir_all(&catalog_dir).unwrap();
    std::fs::write(catalog_dir.join("index.json"), output.index).unwrap();

    let app = router(ServerConfig::new(&fixture.data, &fixture.media)).unwrap();

    let (status, _, body) = send(&app, Method::GET, "/", Vec::new()).await;
    assert_eq!(status, StatusCode::OK);
    let root = html(&body);
    assert!(root.contains("<h1>Years</h1>"));
    assert!(root.contains("No media found yet."));
    assert!(!root.contains("<h1>Recent</h1>"));
    assert!(!root.contains("recent-grid"));
}

#[tokio::test]
async fn archive_pages_use_the_hybrid_order_while_events_stay_chronological() {
    let fixture = Fixture::new();
    fixture.write_jpeg_with_exif(
        "2024/05/early.jpg",
        Some(1),
        Some("2024:05:01 10:00:00"),
        false,
    );
    fixture.write_jpeg_with_exif(
        "2024/05/later.jpg",
        Some(1),
        Some("2024:05:02 10:00:00"),
        false,
    );
    fixture.write_jpeg_with_exif(
        "2024/12/end.jpg",
        Some(1),
        Some("2024:12:01 10:00:00"),
        false,
    );
    fixture.write_jpeg_with_exif(
        "2023/12/old.jpg",
        Some(1),
        Some("2023:12:15 10:00:00"),
        false,
    );

    let events_path = fixture.root().join("events.toml");
    std::fs::write(
        &events_path,
        r#"
[[events]]
slug = "spring"
title = "Spring"
start = "2024-05-01 00:00"
end = "2024-12-31 23:59"
"#,
    )
    .unwrap();

    let mut config = fixture.config();
    config.events_file = Some(events_path);
    let outcome = run_with_config(config).unwrap();
    assert!(
        outcome.summary.errors.is_empty(),
        "{:?}",
        outcome.summary.errors
    );

    let app = router(ServerConfig::new(&fixture.data, &fixture.media)).unwrap();

    let (_, _, body) = send(&app, Method::GET, "/", Vec::new()).await;
    let root = html(&body);
    assert!(
        root.find("href=\"/2024/\"").unwrap() < root.find("href=\"/2023/\"").unwrap(),
        "root years must be newest first"
    );

    let (_, _, body) = send(&app, Method::GET, "/2024/", Vec::new()).await;
    let year = html(&body);
    assert!(
        year.find("href=\"/2024/05/\"").unwrap() < year.find("href=\"/2024/12/\"").unwrap(),
        "year months must ascend"
    );

    let (_, _, body) = send(&app, Method::GET, "/2024/05/", Vec::new()).await;
    let month = html(&body);
    assert!(
        month.find("href=\"/media/2024/05/early.jpg\"").unwrap()
            < month.find("href=\"/media/2024/05/later.jpg\"").unwrap(),
        "month items must be oldest first"
    );

    let (_, _, body) = send(&app, Method::GET, "/events/spring/", Vec::new()).await;
    let event = html(&body);
    assert!(
        event.find("May 2024").unwrap() < event.find("December 2024").unwrap(),
        "event month groups must stay oldest first"
    );
    assert!(
        event.find("href=\"/media/2024/05/early.jpg\"").unwrap()
            < event.find("href=\"/media/2024/05/later.jpg\"").unwrap(),
        "event items must stay oldest first"
    );
    assert!(!event.contains("old.jpg"));

    let index: Value = serde_json::from_slice(
        &send(&app, Method::GET, "/catalog/index.json", Vec::new())
            .await
            .2,
    )
    .unwrap();
    let years = index["years"]
        .as_array()
        .unwrap()
        .iter()
        .map(|year| year["year"].as_i64().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(years, vec![2024, 2023]);
    let months = index["years"][0]["months"]
        .as_array()
        .unwrap()
        .iter()
        .map(|month| month["month"].as_u64().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(months, vec![5, 12]);

    let (_, _, shard_body) = send(&app, Method::GET, "/catalog/2024.json", Vec::new()).await;
    let shard: Value = serde_json::from_slice(&shard_body).unwrap();
    let paths = shard["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["path"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        paths,
        vec!["2024/05/early.jpg", "2024/05/later.jpg", "2024/12/end.jpg"]
    );
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
