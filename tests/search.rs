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
