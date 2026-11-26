mod common;

use std::process::Command;

use common::Fixture;
use media_catalog::run_with_config;
use serde_json::Value;

#[test]
fn search_filters_narrow_the_loaded_shards() {
    let node_available = Command::new("node")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);
    if !node_available {
        eprintln!("skipping search JS test: node is unavailable");
        return;
    }

    let fixture = Fixture::new();
    fixture.write_image("2024/05/alpha.jpg", 20, 20, 1);
    fixture.write_image("2024/05/beta.jpg", 20, 20, 2);
    if !fixture.write_video("2023/12/gamma.mp4", 2) {
        eprintln!("skipping search JS test: ffmpeg unavailable");
        return;
    }

    let outcome = run_with_config(fixture.config()).unwrap();
    assert!(
        outcome.summary.errors.is_empty(),
        "{:?}",
        outcome.summary.errors
    );

    let script = format!("{}/tests/js/filter_check.mjs", env!("CARGO_MANIFEST_DIR"));
    let asset = format!("{}/assets/search.js", env!("CARGO_MANIFEST_DIR"));
    let output = Command::new("node")
        .args([
            script.as_str(),
            asset.as_str(),
            fixture.data.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "node failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["total"], 3);
    assert_eq!(result["text"], 1);
    assert_eq!(result["video"], 1);
    assert_eq!(result["year2024"], 2);
    assert_eq!(result["month05"], 2);
    assert_eq!(result["combined"], 1);
    assert_eq!(result["noMatch"], 0);
    assert_eq!(result["restoredText"], "alpha");
    assert_eq!(result["restoredKind"], "video");
    assert_eq!(result["restoredYear"], "2024");
    assert_eq!(result["restoredMonth"], "05");
    assert_eq!(result["captionWithModel"], "24-05-01 10:00 iPhone 15 Pro");
    assert_eq!(result["captionWithoutModel"], "24-05-01 10:00");
    assert_eq!(result["captionEmptyModel"], "24-05-01 10:00");
    assert_eq!(result["metadataPresent"]["data-f-number"], "1.8");
    assert_eq!(result["metadataPresent"]["data-exposure"], "1/125");
    assert_eq!(result["metadataPresent"]["data-iso"], "100");
    assert_eq!(result["metadataPresent"]["data-focal"], "26");
    assert_eq!(result["metadataPresent"]["data-lat"], "52.520008");
    assert_eq!(result["metadataPresent"]["data-lon"], "13.404954");
    assert_eq!(result["metadataAbsent"], serde_json::json!({}));
    assert_eq!(
        result["metadataWithoutLocation"],
        serde_json::json!({"data-iso": "100"})
    );
    assert_eq!(
        result["order"],
        serde_json::json!(["2024/05/alpha.jpg", "2024/05/beta.jpg", "2023/12/gamma.mp4"])
    );
    assert_eq!(result["query"], "q=gamma&kind=video&year=2023&month=12");
}

#[test]
fn search_page_works_without_javascript_and_links_to_browse() {
    let html = media_catalog::render::render_search_shell().unwrap();
    assert!(html.contains("<noscript>"));
    assert!(html.contains("browse the catalog by year"));
    assert!(html.contains("href=\"/\""));
    assert!(html.contains("id=\"filters\""));
    assert!(html.contains("assets/search.js"));
}

#[test]
fn published_json_and_search_data_follow_the_archive_order() {
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
        "2024/12/late.jpg",
        Some(1),
        Some("2024:12:02 10:00:00"),
        false,
    );
    fixture.write_jpeg_with_exif(
        "2023/12/old.jpg",
        Some(1),
        Some("2023:12:31 10:00:00"),
        false,
    );

    let outcome = run_with_config(fixture.config()).unwrap();
    assert!(
        outcome.summary.errors.is_empty(),
        "{:?}",
        outcome.summary.errors
    );

    let index: Value = serde_json::from_str(&fixture.read_json("catalog/index.json")).unwrap();
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

    // Search concatenates shard items in index order, so this is the order its
    // filters preserve.
    let mut paths = Vec::new();
    for year in index["years"].as_array().unwrap() {
        let shard_path = year["shard"].as_str().unwrap();
        let shard: Value = serde_json::from_str(&fixture.read_json(shard_path)).unwrap();
        for item in shard["items"].as_array().unwrap() {
            paths.push(item["path"].as_str().unwrap().to_string());
        }
    }
    assert_eq!(
        paths,
        vec![
            "2024/05/early.jpg",
            "2024/05/later.jpg",
            "2024/12/late.jpg",
            "2023/12/old.jpg"
        ]
    );
}

#[test]
fn json_index_and_shards_are_shareable_without_extra_requests() {
    let fixture = Fixture::new();
    fixture.write_image("2024/05/a.jpg", 20, 20, 1);
    run_with_config(fixture.config()).unwrap();

    let index: Value = serde_json::from_str(&fixture.read_json("catalog/index.json")).unwrap();
    assert_eq!(index["total_items"], 1);
    assert_eq!(index["years"][0]["shard"], "catalog/2024.json");
    assert_eq!(index["years"][0]["months"][0]["month"], 5);

    let shard: Value = serde_json::from_str(&fixture.read_json("catalog/2024.json")).unwrap();
    let item = &shard["items"][0];
    assert_eq!(item["path"], "2024/05/a.jpg");
    assert!(item["media_url"].as_str().unwrap().starts_with("/media/"));
    assert!(item["thumb_url"].as_str().unwrap().starts_with("/thumbs/"));
}
