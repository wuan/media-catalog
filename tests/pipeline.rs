mod common;

use std::collections::HashSet;
use std::time::Duration;

use common::{set_readable, Fixture};
use media_catalog::fingerprint;
use media_catalog::lock::BuildLock;
use media_catalog::state::StateDb;
use media_catalog::{json, render, run_with_config};
use rusqlite::params;
use serde_json::Value;

fn rendered_month(fixture: &Fixture, year: i32, month: u32) -> String {
    let catalog = json::load_catalog(&fixture.data.join("catalog")).unwrap();
    render::render_month(&catalog, year, month)
        .unwrap()
        .expect("month page")
}

#[test]
fn first_run_builds_output_and_second_run_is_idempotent() {
    let fixture = Fixture::new();
    fixture.write_image("2024/05/a.jpg", 40, 20, 1);
    fixture.write_image("2024/05/b.jpg", 20, 40, 2);
    fixture.write_image("2024/06/c.jpg", 30, 30, 3);

    let first = run_with_config(fixture.config()).unwrap();
    assert!(!first.blocked);
    assert!(
        first.summary.errors.is_empty(),
        "{:?}",
        first.summary.errors
    );
    assert_eq!(first.summary.scanned, 3);
    assert_eq!(first.summary.new, 3);
    assert_eq!(first.summary.changed, 0);
    assert_eq!(first.summary.unchanged, 0);
    assert_eq!(first.summary.thumbnails_generated, 3);
    assert_eq!(first.summary.thumbnails_skipped, 0);
    assert_eq!(first.summary.json_files_written, 2); // index plus one year shard

    assert!(fixture.data_path("catalog/index.json").exists());
    assert!(fixture.data_path("catalog/2024.json").exists());
    for obsolete in [
        "index.html",
        "2024/index.html",
        "2024/05/index.html",
        "2024/06/index.html",
        "search.html",
        "assets/site.css",
        "assets/search.js",
    ] {
        assert!(
            !fixture.data_path(obsolete).exists(),
            "generator wrote obsolete {obsolete}"
        );
    }

    for rel in ["2024/05/a.jpg", "2024/05/b.jpg", "2024/06/c.jpg"] {
        let thumb = fixture.thumb_path(rel);
        assert!(thumb.exists(), "missing thumbnail {}", thumb.display());
        let image = image::open(&thumb).unwrap();
        assert_eq!(image.width(), 512);
        assert_eq!(image.height(), 512);
    }

    let index_before = fixture.read_json("catalog/index.json");
    let shard_before = fixture.read_json("catalog/2024.json");
    let month_before = rendered_month(&fixture, 2024, 5);
    let thumbs_before: Vec<_> = ["2024/05/a.jpg", "2024/05/b.jpg", "2024/06/c.jpg"]
        .into_iter()
        .map(|rel| fixture.thumb_mtime(rel))
        .collect();

    let second = run_with_config(fixture.config()).unwrap();
    assert!(second.summary.errors.is_empty());
    assert_eq!(second.summary.scanned, 3);
    assert_eq!(second.summary.new, 0);
    assert_eq!(second.summary.changed, 0);
    assert_eq!(second.summary.unchanged, 3);
    assert_eq!(second.summary.thumbnails_generated, 0);
    assert_eq!(second.summary.thumbnails_skipped, 3);

    assert_eq!(fixture.read_json("catalog/index.json"), index_before);
    assert_eq!(fixture.read_json("catalog/2024.json"), shard_before);
    assert_eq!(rendered_month(&fixture, 2024, 5), month_before);
    let thumbs_after: Vec<_> = ["2024/05/a.jpg", "2024/05/b.jpg", "2024/06/c.jpg"]
        .into_iter()
        .map(|rel| fixture.thumb_mtime(rel))
        .collect();
    assert_eq!(thumbs_before, thumbs_after);

    let state = StateDb::open(&fixture.state_db()).unwrap();
    assert_eq!(state.item_count().unwrap(), 3);
}

#[test]
fn v1_state_migrates_and_reprobes_unchanged_items_once() {
    let fixture = Fixture::new();
    fixture.write_image("2024/05/a.jpg", 20, 20, 1);

    let source = fixture.media_path("2024/05/a.jpg");
    let metadata = std::fs::metadata(&source).unwrap();
    let size = metadata.len();
    let mtime_ns = fingerprint::mtime_nanos(&metadata);
    let file_fingerprint = fingerprint::fingerprint(size, mtime_ns);

    std::fs::create_dir_all(&fixture.state).unwrap();
    let conn = rusqlite::Connection::open(fixture.state_db()).unwrap();
    conn.execute_batch(
        "CREATE TABLE items (
            path          TEXT PRIMARY KEY,
            year          INTEGER NOT NULL,
            month         INTEGER NOT NULL,
            kind          TEXT NOT NULL,
            size          INTEGER NOT NULL,
            mtime         INTEGER NOT NULL,
            fingerprint   TEXT NOT NULL,
            width         INTEGER,
            height        INTEGER,
            duration_ms   INTEGER,
            captured_at   TEXT,
            thumb_path    TEXT,
            thumb_version TEXT,
            thumb_status  TEXT NOT NULL,
            error         TEXT,
            last_seen_run INTEGER NOT NULL
        );
        CREATE INDEX items_month ON items(year, month);
        CREATE TABLE runs (
            id          INTEGER PRIMARY KEY,
            started_at  TEXT,
            finished_at TEXT,
            status      TEXT,
            item_count  INTEGER,
            error_count INTEGER
        );
        CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT);
        INSERT INTO meta (key, value) VALUES ('schema_version', '1');",
    )
    .unwrap();
    conn.execute(
        "INSERT INTO items
            (path, year, month, kind, size, mtime, fingerprint, width, height,
             duration_ms, captured_at, thumb_path, thumb_version, thumb_status,
             error, last_seen_run)
         VALUES (?1, 2024, 5, 'image', ?2, ?3, ?4, 10, 10, NULL,
                 '2000-01-01 00:00:00', NULL, NULL, 'pending', NULL, 0)",
        params!["2024/05/a.jpg", size as i64, mtime_ns, file_fingerprint,],
    )
    .unwrap();
    drop(conn);

    let first = run_with_config(fixture.config()).unwrap();
    assert_eq!(first.summary.new, 0);
    assert_eq!(first.summary.changed, 0);
    assert_eq!(first.summary.unchanged, 1);
    assert_eq!(first.summary.probed, 1);
    assert_eq!(first.summary.thumbnails_generated, 1);

    let state = StateDb::open(&fixture.state_db()).unwrap();
    let item = state.load_items().unwrap().remove("2024/05/a.jpg").unwrap();
    assert_eq!(item.width, Some(20));
    assert_eq!(item.height, Some(20));
    assert!(item.captured_at.is_some());
    assert_eq!(item.thumb_status, media_catalog::state::ThumbStatus::Ok);

    let second = run_with_config(fixture.config()).unwrap();
    assert_eq!(second.summary.probed, 0);
    assert_eq!(second.summary.thumbnails_generated, 0);
    assert_eq!(second.summary.thumbnails_skipped, 1);
}

#[test]
fn changed_item_regenerates_only_that_thumbnail() {
    let fixture = Fixture::new();
    fixture.write_image("2024/05/a.jpg", 40, 20, 1);
    fixture.write_image("2024/05/b.jpg", 20, 40, 2);

    let first = run_with_config(fixture.config()).unwrap();
    assert_eq!(first.summary.thumbnails_generated, 2);
    let shard_before = fixture.read_json("catalog/2024.json");
    let unchanged_before = fixture.thumb_mtime("2024/05/b.jpg");
    let changed_before = fixture.thumb_mtime("2024/05/a.jpg");

    std::thread::sleep(Duration::from_millis(20));
    fixture.write_image("2024/05/a.jpg", 40, 20, 9);
    fixture.write_image("2024/05/new.jpg", 25, 25, 7);

    let second = run_with_config(fixture.config()).unwrap();
    assert_eq!(second.summary.new, 1);
    assert_eq!(second.summary.changed, 1);
    assert_eq!(second.summary.unchanged, 1);
    assert_eq!(second.summary.thumbnails_generated, 2);
    assert_eq!(second.summary.thumbnails_skipped, 1);
    assert!(fixture.thumb_path("2024/05/new.jpg").exists());
    assert_eq!(fixture.thumb_mtime("2024/05/b.jpg"), unchanged_before);
    assert_ne!(fixture.thumb_mtime("2024/05/a.jpg"), changed_before);

    let shard_after = fixture.read_json("catalog/2024.json");
    assert_ne!(shard_before, shard_after);
    let before: Value = serde_json::from_str(&shard_before).unwrap();
    let after: Value = serde_json::from_str(&shard_after).unwrap();
    let find = |json: &Value, path: &str| {
        json["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["path"] == path)
            .unwrap()
            .clone()
    };
    assert_ne!(
        find(&before, "2024/05/a.jpg")["thumb_url"],
        find(&after, "2024/05/a.jpg")["thumb_url"]
    );
    assert_eq!(
        find(&before, "2024/05/b.jpg")["thumb_url"],
        find(&after, "2024/05/b.jpg")["thumb_url"]
    );
}

#[test]
fn deleted_items_disappear_and_prune_is_opt_in() {
    let fixture = Fixture::new();
    fixture.write_image("2024/05/a.jpg", 20, 20, 1);
    fixture.write_image("2024/05/b.jpg", 20, 20, 2);
    fixture.write_image("2024/06/c.jpg", 20, 20, 3);
    run_with_config(fixture.config()).unwrap();

    fixture.remove("2024/05/b.jpg");
    let without_prune = run_with_config(fixture.config()).unwrap();
    assert_eq!(without_prune.summary.deleted, 1);
    assert_eq!(without_prune.summary.pruned, 0);
    assert!(!fixture
        .read_json("catalog/2024.json")
        .contains("2024/05/b.jpg"));
    assert!(!rendered_month(&fixture, 2024, 5).contains("2024/05/b.jpg"));
    assert!(fixture.thumb_path("2024/05/b.jpg").exists());

    let mut prune_config = fixture.config();
    prune_config.prune = true;
    let with_prune = run_with_config(prune_config).unwrap();
    assert_eq!(with_prune.summary.pruned, 1);
    assert!(!fixture.thumb_path("2024/05/b.jpg").exists());
    assert!(fixture.thumb_path("2024/05/a.jpg").exists());
    assert!(fixture.thumb_path("2024/06/c.jpg").exists());
    assert!(fixture.data_path("catalog/index.json").exists());
}

#[test]
fn mount_drop_blocks_cleanup_until_overridden() {
    let fixture = Fixture::new();
    for index in 0..6 {
        fixture.write_image(&format!("2024/05/{index}.jpg"), 20, 20, index);
    }
    run_with_config(fixture.config()).unwrap();

    for index in 1..6 {
        fixture.remove(&format!("2024/05/{index}.jpg"));
    }

    let mut blocked_config = fixture.config();
    blocked_config.prune = true;
    let blocked = run_with_config(blocked_config).unwrap();
    assert!(blocked.blocked);
    assert!(blocked.summary.mount_warning.is_some());
    assert_eq!(blocked.summary.pruned, 0);
    assert_eq!(blocked.summary.deleted, 0);
    assert!(fixture.thumb_path("2024/05/5.jpg").exists());
    assert_eq!(
        StateDb::open(&fixture.state_db())
            .unwrap()
            .item_count()
            .unwrap(),
        6
    );

    let mut override_config = fixture.config();
    override_config.prune = true;
    override_config.allow_mount_drop = true;
    let overridden = run_with_config(override_config).unwrap();
    assert!(!overridden.blocked);
    assert_eq!(overridden.summary.pruned, 5);
    assert_eq!(overridden.summary.deleted, 5);
    assert!(fixture.thumb_path("2024/05/0.jpg").exists());
    assert!(!fixture.thumb_path("2024/05/5.jpg").exists());
    assert_eq!(
        StateDb::open(&fixture.state_db())
            .unwrap()
            .item_count()
            .unwrap(),
        1
    );
}

#[test]
fn dry_run_writes_nothing() {
    let fixture = Fixture::new();
    fixture.write_image("2024/05/a.jpg", 20, 20, 1);
    fixture.write_image("2024/05/b.jpg", 20, 20, 2);
    let mut config = fixture.config();
    config.dry_run = true;

    let outcome = run_with_config(config).unwrap();
    assert!(!outcome.blocked);
    assert_eq!(outcome.summary.scanned, 2);
    assert_eq!(outcome.summary.new, 2);
    assert!(outcome.summary.dry_run);
    assert!(!fixture.data.exists());
    assert!(!fixture.state.exists());
    assert!(!fixture.state_db().exists());
}

#[cfg(unix)]
#[test]
fn failed_thumbnail_is_retried_without_regenerating_completed_work() {
    let fixture = Fixture::new();
    fixture.write_image("2024/05/a.jpg", 20, 20, 1);
    fixture.write_image("2024/05/b.jpg", 20, 20, 2);
    fixture.write_image("2024/05/locked.jpg", 20, 20, 3);
    set_readable(&fixture.media_path("2024/05/locked.jpg"), false);

    let first = run_with_config(fixture.config()).unwrap();
    assert_eq!(first.summary.thumbnails_generated, 2);
    assert_eq!(first.summary.thumbnails_failed, 1);
    assert_eq!(first.summary.errors.len(), 1);
    assert!(first.summary.errors[0].path.ends_with("locked.jpg"));
    assert!(fixture
        .read_json("catalog/2024.json")
        .contains("\"thumb_available\": false"));

    let a_before = fixture.thumb_mtime("2024/05/a.jpg");
    let b_before = fixture.thumb_mtime("2024/05/b.jpg");

    set_readable(&fixture.media_path("2024/05/locked.jpg"), true);
    let second = run_with_config(fixture.config()).unwrap();
    assert!(
        second.summary.errors.is_empty(),
        "{:?}",
        second.summary.errors
    );
    assert_eq!(second.summary.changed, 0);
    assert_eq!(second.summary.unchanged, 3);
    assert_eq!(second.summary.thumbnails_generated, 1);
    assert_eq!(second.summary.thumbnails_skipped, 2);
    assert_eq!(fixture.thumb_mtime("2024/05/a.jpg"), a_before);
    assert_eq!(fixture.thumb_mtime("2024/05/b.jpg"), b_before);
    assert!(fixture.thumb_path("2024/05/locked.jpg").exists());
}

#[cfg(unix)]
#[test]
fn unchanged_items_do_not_reopen_their_sources() {
    let fixture = Fixture::new();
    fixture.write_image("2024/05/a.jpg", 20, 20, 1);
    fixture.write_image("2024/05/b.jpg", 20, 20, 2);
    run_with_config(fixture.config()).unwrap();

    set_readable(&fixture.media_path("2024/05/a.jpg"), false);
    set_readable(&fixture.media_path("2024/05/b.jpg"), false);

    let second = run_with_config(fixture.config()).unwrap();
    assert!(
        second.summary.errors.is_empty(),
        "{:?}",
        second.summary.errors
    );
    assert_eq!(second.summary.unchanged, 2);
    assert_eq!(second.summary.thumbnails_generated, 0);
    assert_eq!(second.summary.thumbnails_skipped, 2);

    set_readable(&fixture.media_path("2024/05/a.jpg"), true);
    set_readable(&fixture.media_path("2024/05/b.jpg"), true);
}

#[test]
fn heic_is_unavailable_and_metadata_is_private() {
    let fixture = Fixture::new();
    fixture.write_jpeg_with_exif(
        "2024/05/geo.jpg",
        Some(6),
        Some("2024:05:01 10:11:12"),
        true,
    );
    fixture.write_bytes("2024/05/phone.heic", b"fake heic");

    let outcome = run_with_config(fixture.config()).unwrap();
    assert!(!outcome.blocked);
    assert!(
        outcome.summary.errors.is_empty(),
        "{:?}",
        outcome.summary.errors
    );
    assert_eq!(outcome.summary.thumbnails_generated, 1);
    assert_eq!(outcome.summary.thumbnails_unsupported, 1);

    let shard = fixture.read_json("catalog/2024.json");
    let lower = shard.to_ascii_lowercase();
    for forbidden in ["gps", "latitude", "longitude", "serial"] {
        assert!(
            !lower.contains(forbidden),
            "JSON contained {forbidden}: {shard}"
        );
    }
    assert!(shard.contains("2024-05-01 10:11:12"));
    assert!(shard.contains("\"thumb_available\": false"));

    let html = rendered_month(&fixture, 2024, 5);
    assert!(html.contains("phone.heic"));
    assert!(html.contains("class=\"placeholder\""));
}

#[test]
fn camera_metadata_is_published_and_location_is_opt_in() {
    let fixture = Fixture::new();
    fixture.write_jpeg_with_exif(
        "2024/05/photo.jpg",
        Some(1),
        Some("2024:05:01 10:11:12"),
        true,
    );
    fixture.write_jpeg_with_exif(
        "2024/05/phone.heic",
        Some(1),
        Some("2024:05:02 10:11:12"),
        true,
    );

    let first = run_with_config(fixture.config()).unwrap();
    assert!(!first.blocked);
    assert!(
        first.summary.errors.is_empty(),
        "{:?}",
        first.summary.errors
    );
    assert_eq!(first.summary.probed, 2);
    assert_eq!(first.summary.thumbnails_generated, 1);
    assert_eq!(first.summary.thumbnails_unsupported, 1);

    let shard: Value = serde_json::from_str(&fixture.read_json("catalog/2024.json")).unwrap();
    let items = shard["items"].as_array().unwrap();
    let photo = items
        .iter()
        .find(|item| item["path"] == "2024/05/photo.jpg")
        .unwrap();
    assert_eq!(photo["f_number"], 1.8);
    assert_eq!(photo["exposure_time"], "1/125");
    assert_eq!(photo["iso"], 100);
    assert_eq!(photo["focal_length_mm"], 26.0);
    assert!(photo.get("latitude").is_none());
    assert!(photo.get("longitude").is_none());

    let heic = items
        .iter()
        .find(|item| item["path"] == "2024/05/phone.heic")
        .unwrap();
    assert_eq!(heic["thumb_available"], false);
    assert_eq!(heic["captured_at"], "2024-05-02 10:11:12");
    assert_eq!(heic["iso"], 100);
    assert!(heic.get("latitude").is_none());

    let html = rendered_month(&fixture, 2024, 5);
    assert!(html.contains("data-f-number=\"1.8\""));
    assert!(html.contains("data-exposure=\"1/125\""));
    assert!(html.contains("data-iso=\"100\""));
    assert!(html.contains("data-focal=\"26\""));
    assert!(!html.contains("data-lat="));

    let mut config = fixture.config();
    config.include_location = true;
    let second = run_with_config(config).unwrap();
    assert!(
        second.summary.errors.is_empty(),
        "{:?}",
        second.summary.errors
    );
    assert_eq!(second.summary.probed, 0);

    let shard: Value = serde_json::from_str(&fixture.read_json("catalog/2024.json")).unwrap();
    let items = shard["items"].as_array().unwrap();
    let photo = items
        .iter()
        .find(|item| item["path"] == "2024/05/photo.jpg")
        .unwrap();
    assert_eq!(photo["latitude"], 52.52);
    assert_eq!(photo["longitude"], 13.405);

    let html = rendered_month(&fixture, 2024, 5);
    assert!(html.contains("data-lat=\"52.52\""));
    assert!(html.contains("data-lon=\"13.405\""));
}

#[test]
fn location_preview_is_advertised_only_when_enabled() {
    let fixture = Fixture::new();
    fixture.write_jpeg_with_exif(
        "2024/05/photo.jpg",
        Some(1),
        Some("2024:05:01 10:11:12"),
        true,
    );

    let mut config = fixture.config();
    config.include_location = true;
    config.location_preview = true;
    let enabled = run_with_config(config).unwrap();
    assert!(
        enabled.summary.errors.is_empty(),
        "{:?}",
        enabled.summary.errors
    );

    let index: Value = serde_json::from_str(&fixture.read_json("catalog/index.json")).unwrap();
    assert_eq!(index["location_preview"], true);
    let html = rendered_month(&fixture, 2024, 5);
    assert!(html.contains("<body data-location-preview=\"true\">"));
    assert!(html.contains("data-lat=\"52.52\""));

    let mut config = fixture.config();
    config.include_location = true;
    let disabled = run_with_config(config).unwrap();
    assert!(
        disabled.summary.errors.is_empty(),
        "{:?}",
        disabled.summary.errors
    );

    let index: Value = serde_json::from_str(&fixture.read_json("catalog/index.json")).unwrap();
    assert!(index.get("location_preview").is_none());
    let html = rendered_month(&fixture, 2024, 5);
    assert!(!html.contains("data-location-preview"));
}

#[test]
fn video_location_is_published_when_enabled() {
    let fixture = Fixture::new();
    if !fixture.write_video_with_location("2024/05/clip.mp4", 1, "+52.5200+013.4050/") {
        eprintln!("skipping video location test: ffmpeg unavailable");
        return;
    }

    let mut config = fixture.config();
    config.include_location = true;
    let outcome = run_with_config(config).unwrap();
    assert!(
        outcome.summary.errors.is_empty(),
        "{:?}",
        outcome.summary.errors
    );

    let shard: Value = serde_json::from_str(&fixture.read_json("catalog/2024.json")).unwrap();
    let video = shard["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["path"] == "2024/05/clip.mp4")
        .unwrap();
    assert_eq!(video["latitude"], 52.52);
    assert_eq!(video["longitude"], 13.405);
}

#[test]
fn video_items_receive_posters_and_duration() {
    let fixture = Fixture::new();
    fixture.write_image("2024/05/photo.jpg", 20, 20, 1);
    if !fixture.write_video("2024/05/clip.mp4", 3) {
        eprintln!("skipping video pipeline test: ffmpeg unavailable");
        return;
    }

    let outcome = run_with_config(fixture.config()).unwrap();
    assert!(
        outcome.summary.errors.is_empty(),
        "{:?}",
        outcome.summary.errors
    );
    assert_eq!(outcome.summary.videos, 1);
    assert_eq!(outcome.summary.thumbnails_generated, 2);

    let thumb = image::open(fixture.thumb_path("2024/05/clip.mp4")).unwrap();
    assert_eq!(thumb.width(), 512);
    assert_eq!(thumb.height(), 512);

    let shard = fixture.read_json("catalog/2024.json");
    assert!(shard.contains("\"kind\": \"video\""));
    assert!(shard.contains("\"duration_ms\""));
    let json: Value = serde_json::from_str(&shard).unwrap();
    let video = json["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["path"] == "2024/05/clip.mp4")
        .unwrap();
    assert!(video["duration_ms"].as_u64().unwrap() > 2000);
}

#[test]
fn lock_blocks_a_concurrent_run_and_is_released() {
    let fixture = Fixture::new();
    fixture.write_image("2024/05/a.jpg", 20, 20, 1);
    let lock = BuildLock::acquire(&fixture.state_db()).unwrap();

    let error = run_with_config(fixture.config()).unwrap_err();
    assert!(format!("{error:#}").contains("another build is already running"));
    assert!(!fixture.data.exists());

    drop(lock);
    run_with_config(fixture.config()).unwrap();
    assert!(fixture.data_path("catalog/index.json").exists());
}

#[test]
fn writes_only_the_data_and_state_trees() {
    let fixture = Fixture::new();
    fixture.write_image("2024/05/a.jpg", 20, 20, 1);
    fixture.write_image("2024/05/b.jpg", 20, 20, 2);

    let before = snapshot_files(fixture.root());
    run_with_config(fixture.config()).unwrap();
    let after = snapshot_files(fixture.root());

    for (path, bytes, mtime) in &before {
        let current = after
            .iter()
            .find(|entry| &entry.0 == path)
            .unwrap_or_else(|| panic!("original disappeared: {}", path.display()));
        assert_eq!(&current.1, bytes, "original modified: {}", path.display());
        assert_eq!(
            &current.2,
            mtime,
            "original mtime changed: {}",
            path.display()
        );
    }

    let originals: HashSet<_> = before.iter().map(|(path, _, _)| path.clone()).collect();
    for (path, _, _) in &after {
        if originals.contains(path) {
            continue;
        }
        let rel = path.strip_prefix(fixture.root()).unwrap().to_string_lossy();
        assert!(
            rel.starts_with("data/") || rel.starts_with("state/"),
            "unexpected file outside data/state: {rel}"
        );
    }
}

fn snapshot_files(
    root: &std::path::Path,
) -> Vec<(std::path::PathBuf, Vec<u8>, std::time::SystemTime)> {
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(root).follow_links(false) {
        let entry = entry.unwrap();
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path().to_path_buf();
        let bytes = std::fs::read(&path).unwrap();
        let mtime = std::fs::metadata(&path).unwrap().modified().unwrap();
        files.push((path, bytes, mtime));
    }
    files.sort_by(|left, right| left.0.cmp(&right.0));
    files
}

#[test]
fn invalid_events_fail_before_scanning_and_write_nothing() {
    let fixture = Fixture::new();
    fixture.write_image("2024/05/a.jpg", 20, 20, 1);
    let events_path = fixture.root().join("events.toml");
    std::fs::write(
        &events_path,
        r#"
[[events]]
slug = "bad"
title = "Bad range"
start = "2024-05-02 09:00"
end = "2024-05-01 09:00"
"#,
    )
    .unwrap();

    let mut config = fixture.config();
    config.events_file = Some(events_path);
    let error = run_with_config(config).unwrap_err();
    assert!(error.to_string().contains("after it ends"), "{error:#}");
    assert!(!fixture.data.exists());
    assert!(!fixture.state_db().exists());
}

#[test]
fn events_load_through_media_catalog_toml() {
    use clap::Parser;
    use media_catalog::config::{Cli, Config};

    let fixture = Fixture::new();
    fixture.write_jpeg_with_exif("2024/05/a.jpg", Some(1), Some("2024:05:30 10:00:00"), false);
    let events_path = fixture.root().join("events.toml");
    std::fs::write(
        &events_path,
        r#"
[[events]]
slug = "may-trip"
title = "May trip"
start = "2024-05-28 09:00"
end = "2024-06-05 18:30"
"#,
    )
    .unwrap();
    let config_path = fixture.root().join("media-catalog.toml");
    std::fs::write(
        &config_path,
        format!(
            "media_root = \"{}\"\ndata_root = \"{}\"\nstate_root = \"{}\"\nevents_file = \"{}\"\njobs = 2\nquiet = true\n",
            fixture.media.display(),
            fixture.data.display(),
            fixture.state.display(),
            events_path.display()
        ),
    )
    .unwrap();

    let cli = Cli::parse_from(["media-catalog", "--config", config_path.to_str().unwrap()]);
    let config = Config::load(cli).unwrap();
    assert_eq!(config.events_file.as_deref(), Some(events_path.as_path()));

    let outcome = run_with_config(config).unwrap();
    assert_eq!(outcome.summary.event_counts.len(), 1);
    assert_eq!(outcome.summary.event_counts[0].slug, "may-trip");
    assert_eq!(outcome.summary.event_counts[0].count, 1);
    assert!(fixture.read_json("catalog/index.json").contains("may-trip"));
}

#[test]
fn generator_ignores_server_keys_in_media_catalog_toml() {
    use clap::Parser;
    use media_catalog::config::{Cli, Config};

    let fixture = Fixture::new();
    fixture.write_image("2024/05/a.jpg", 20, 20, 1);
    let config_path = fixture.root().join("media-catalog.toml");
    std::fs::write(
        &config_path,
        format!(
            "media_root = \"{}\"\ndata_root = \"{}\"\nstate_root = \"{}\"\nbind = \"127.0.0.1:8080\"\nmax_streams = 2\njobs = 2\nquiet = true\n",
            fixture.media.display(),
            fixture.data.display(),
            fixture.state.display()
        ),
    )
    .unwrap();

    let cli = Cli::parse_from(["media-catalog", "--config", config_path.to_str().unwrap()]);
    let config = Config::load(cli).unwrap();
    let outcome = run_with_config(config).unwrap();
    assert_eq!(outcome.summary.scanned, 1);
    assert!(
        outcome.summary.errors.is_empty(),
        "{:?}",
        outcome.summary.errors
    );
    assert!(fixture.data_path("catalog/index.json").exists());
}

#[test]
fn catalogs_without_events_omit_event_fields_and_load_as_empty() {
    let fixture = Fixture::new();
    fixture.write_image("2024/05/a.jpg", 20, 20, 1);
    fixture.write_image("2024/06/b.jpg", 20, 20, 2);

    run_with_config(fixture.config()).unwrap();
    let index_before = fixture.read_json("catalog/index.json");
    let shard_2024_before = fixture.read_json("catalog/2024.json");
    assert!(!index_before.contains("\"events\""));
    assert!(!shard_2024_before.contains("\"events\""));

    let catalog = json::load_catalog(&fixture.data.join("catalog")).unwrap();
    assert!(catalog.events.is_empty());
    for year in &catalog.years {
        for month in &year.months {
            for item in &month.items {
                assert!(item.events.is_empty());
            }
        }
    }

    run_with_config(fixture.config()).unwrap();
    assert_eq!(fixture.read_json("catalog/index.json"), index_before);
    assert_eq!(fixture.read_json("catalog/2024.json"), shard_2024_before);
}

#[test]
fn prune_never_touches_files_outside_the_thumbnail_tree() {
    let fixture = Fixture::new();
    fixture.write_image("2024/05/a.jpg", 20, 20, 1);
    run_with_config(fixture.config()).unwrap();
    // Simulate obsolete files left in the data root after upgrading from the
    // static tree; prune must stay confined to the thumbnail tree.
    std::fs::write(fixture.data_path("index.html"), "legacy").unwrap();
    std::fs::write(fixture.data_path("search.html"), "legacy").unwrap();
    let outside = HashSet::from([
        fixture.data_path("index.html"),
        fixture.data_path("search.html"),
        fixture.data_path("catalog/index.json"),
        fixture.data_path("catalog/2024.json"),
    ]);

    let mut config = fixture.config();
    config.prune = true;
    run_with_config(config).unwrap();
    for path in outside {
        assert!(path.exists(), "prune removed {}", path.display());
    }
}

#[test]
fn events_only_republishes_from_state_without_mutating_state_rows() {
    let fixture = Fixture::new();
    fixture.write_jpeg_with_exif("2024/05/a.jpg", Some(1), Some("2024:05:30 10:00:00"), false);
    let events_path = fixture.root().join("events.toml");
    std::fs::write(
        &events_path,
        r#"
[[events]]
slug = "may-trip"
title = "May trip"
start = "2024-05-28 09:00"
end = "2024-06-05 18:30"
"#,
    )
    .unwrap();

    let mut config = fixture.config();
    config.events_file = Some(events_path.clone());
    let full = run_with_config(config).unwrap();
    assert!(full.summary.errors.is_empty(), "{:?}", full.summary.errors);
    assert_eq!(full.summary.event_counts.len(), 1);
    assert_eq!(full.summary.event_counts[0].slug, "may-trip");

    // Capture state rows before the events-only run.
    let state_before = StateDb::open(&fixture.state_db()).unwrap();
    let items_before = state_before.load_items().unwrap();
    let item_before = items_before.get("2024/05/a.jpg").unwrap();
    let thumb_status_before = item_before.thumb_status;
    let last_seen_before = item_before.last_seen_run;
    drop(state_before);

    // Edit the events file to add a new event.
    std::fs::write(
        &events_path,
        r#"
[[events]]
slug = "may-trip"
title = "May trip renamed"
start = "2024-05-28 09:00"
end = "2024-06-05 18:30"

[[events]]
slug = "june-trip"
title = "June trip"
start = "2024-06-01 00:00"
end = "2024-06-30 23:59"
"#,
    )
    .unwrap();

    let mut events_config = fixture.config();
    events_config.events_file = Some(events_path);
    events_config.events_only = true;
    let events_run = run_with_config(events_config).unwrap();
    assert!(events_run.summary.events_only);
    assert_eq!(events_run.summary.event_counts.len(), 2);
    assert!(events_run
        .summary
        .event_counts
        .iter()
        .any(|e| e.slug == "june-trip"));

    // The republished index carries the new event.
    let index = fixture.read_json("catalog/index.json");
    assert!(
        index.contains("june-trip"),
        "index missing june-trip: {index}"
    );
    assert!(index.contains("May trip renamed"));

    // State rows are unchanged: thumb status and last_seen_run preserved.
    let state_after = StateDb::open(&fixture.state_db()).unwrap();
    let items_after = state_after.load_items().unwrap();
    let item_after = items_after.get("2024/05/a.jpg").unwrap();
    assert_eq!(item_after.thumb_status, thumb_status_before);
    assert_eq!(item_after.last_seen_run, last_seen_before);
    assert_eq!(state_after.item_count().unwrap(), 1);
}

#[test]
fn events_only_succeeds_with_media_root_removed_and_byte_identical_output() {
    let fixture = Fixture::new();
    fixture.write_image("2024/05/a.jpg", 20, 20, 1);
    fixture.write_image("2024/06/b.jpg", 20, 20, 2);

    let full = run_with_config(fixture.config()).unwrap();
    assert!(full.summary.errors.is_empty(), "{:?}", full.summary.errors);
    let index_before = fixture.read_json("catalog/index.json");
    let shard_before = fixture.read_json("catalog/2024.json");

    // Remove the media root entirely.
    std::fs::remove_dir_all(&fixture.media).unwrap();
    assert!(!fixture.media.exists());

    let mut config = fixture.config();
    config.events_only = true;
    let events_run = run_with_config(config).unwrap();
    assert!(events_run.summary.events_only);
    assert!(
        events_run.summary.errors.is_empty(),
        "{:?}",
        events_run.summary.errors
    );

    // Byte-identical output.
    assert_eq!(fixture.read_json("catalog/index.json"), index_before);
    assert_eq!(fixture.read_json("catalog/2024.json"), shard_before);
}

#[test]
fn events_only_without_state_db_fails_and_writes_nothing() {
    let fixture = Fixture::new();
    fixture.write_image("2024/05/a.jpg", 20, 20, 1);

    // No full build: state DB does not exist.
    assert!(!fixture.state_db().exists());

    let mut config = fixture.config();
    config.events_only = true;
    let error = run_with_config(config).unwrap_err();
    let msg = format!("{error:#}");
    assert!(
        msg.contains("full build first") || msg.contains("state database"),
        "{msg}"
    );
    assert!(!fixture.data.exists(), "no data should be written");
    assert!(!fixture.state_db().exists(), "no state should be written");
}

#[test]
fn events_only_dry_run_writes_nothing() {
    let fixture = Fixture::new();
    fixture.write_image("2024/05/a.jpg", 20, 20, 1);
    let events_path = fixture.root().join("events.toml");
    std::fs::write(
        &events_path,
        r#"
[[events]]
slug = "may-trip"
title = "May trip"
start = "2024-05-01 00:00"
end = "2024-05-31 23:59"
"#,
    )
    .unwrap();

    let mut config = fixture.config();
    config.events_file = Some(events_path);
    run_with_config(config).unwrap();

    let index_before = fixture.read_json("catalog/index.json");
    let shard_before = fixture.read_json("catalog/2024.json");

    let mut dry_config = fixture.config();
    dry_config.events_file = Some(fixture.root().join("events.toml"));
    dry_config.events_only = true;
    dry_config.dry_run = true;
    let outcome = run_with_config(dry_config).unwrap();
    assert!(outcome.summary.events_only);
    assert!(outcome.summary.dry_run);
    assert_eq!(outcome.summary.json_files_written, 0);

    // Nothing was written.
    assert_eq!(fixture.read_json("catalog/index.json"), index_before);
    assert_eq!(fixture.read_json("catalog/2024.json"), shard_before);
}

#[test]
fn events_only_with_prune_prunes_nothing() {
    let fixture = Fixture::new();
    fixture.write_image("2024/05/a.jpg", 20, 20, 1);
    run_with_config(fixture.config()).unwrap();
    assert!(fixture.thumb_path("2024/05/a.jpg").exists());

    let mut config = fixture.config();
    config.events_only = true;
    config.prune = true;
    let outcome = run_with_config(config).unwrap();
    assert_eq!(outcome.summary.pruned, 0);
    assert_eq!(outcome.summary.deleted, 0);
    assert!(fixture.thumb_path("2024/05/a.jpg").exists());
}

#[test]
fn events_only_with_invalid_events_fails_before_publishing() {
    let fixture = Fixture::new();
    fixture.write_image("2024/05/a.jpg", 20, 20, 1);
    run_with_config(fixture.config()).unwrap();
    let index_before = fixture.read_json("catalog/index.json");

    let events_path = fixture.root().join("events.toml");
    std::fs::write(
        &events_path,
        r#"
[[events]]
slug = "bad"
title = "Bad range"
start = "2024-05-02 09:00"
end = "2024-05-01 09:00"
"#,
    )
    .unwrap();

    let mut config = fixture.config();
    config.events_file = Some(events_path);
    config.events_only = true;
    let error = run_with_config(config).unwrap_err();
    assert!(error.to_string().contains("after it ends"), "{error:#}");

    // Catalog output unchanged.
    assert_eq!(fixture.read_json("catalog/index.json"), index_before);
}

#[test]
fn events_only_still_holds_the_build_lock() {
    let fixture = Fixture::new();
    fixture.write_image("2024/05/a.jpg", 20, 20, 1);
    run_with_config(fixture.config()).unwrap();

    let lock = BuildLock::acquire(&fixture.state_db()).unwrap();

    let mut config = fixture.config();
    config.events_only = true;
    let error = run_with_config(config).unwrap_err();
    assert!(
        format!("{error:#}").contains("another build is already running"),
        "{error:#}"
    );

    drop(lock);
}

fn shard_item(fixture: &Fixture, path: &str) -> Value {
    let shard: Value = serde_json::from_str(&fixture.read_json("catalog/2024.json")).unwrap();
    shard["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["path"] == path)
        .cloned()
        .unwrap_or_else(|| panic!("{path} missing from shard"))
}

/// Make `path` unreadable; returns false (and restores it) when the current
/// user can still open it, e.g. when running as root.
#[cfg(unix)]
fn lock_source(path: &std::path::Path) -> bool {
    set_readable(path, false);
    if std::fs::File::open(path).is_ok() {
        set_readable(path, true);
        return false;
    }
    true
}

#[test]
fn corrupt_jpeg_is_reported_as_a_grouped_probe_failure() {
    let fixture = Fixture::new();
    fixture.write_image("2024/05/a.jpg", 20, 20, 1);
    fixture.write_image("2024/05/b.jpg", 20, 20, 2);
    fixture.write_bytes("2024/05/broken.jpg", b"this is not a jpeg");

    let outcome = run_with_config(fixture.config()).unwrap();
    let summary = &outcome.summary;
    assert!(summary.has_errors());
    assert_eq!(summary.probes_attempted, 3);
    assert_eq!(summary.probe_failure_count(), 1);
    assert_eq!(summary.probe_failures.len(), 1);
    let group = &summary.probe_failures[0];
    assert_eq!(group.samples, ["2024/05/broken.jpg"]);
    assert!(
        group.cause.to_ascii_lowercase().contains("pars"),
        "expected a decode cause: {}",
        group.cause
    );
    assert!(
        !group.cause.contains("broken.jpg"),
        "cause should not embed the path: {}",
        group.cause
    );
    assert!(summary
        .errors
        .iter()
        .any(|failure| failure.path == "2024/05/broken.jpg"));
    assert!(summary.probe_warning.is_none());

    let rendered = summary.render();
    assert!(rendered.contains("Probe failures: 1 of 3"), "{rendered}");
    assert!(
        rendered.contains(&format!("  - {}: 1 (e.g. 2024/05/broken.jpg)", group.cause)),
        "{rendered}"
    );

    let broken = shard_item(&fixture, "2024/05/broken.jpg");
    assert!(broken
        .get("captured_at")
        .is_some_and(|value| !value.is_null()));
}

#[cfg(unix)]
#[test]
fn systemic_probe_failure_prints_a_warning() {
    let fixture = Fixture::new();
    let paths = [
        "2024/05/a.jpg",
        "2024/05/b.jpg",
        "2024/05/c.jpg",
        "2024/05/d.jpg",
    ];
    for (seed, rel) in paths.iter().enumerate() {
        fixture.write_image(rel, 20, 20, seed as u8);
    }
    for rel in paths {
        if !lock_source(&fixture.media_path(rel)) {
            eprintln!("skipping systemic probe test: unreadable files stay readable");
            return;
        }
    }

    let outcome = run_with_config(fixture.config()).unwrap();
    for rel in paths {
        set_readable(&fixture.media_path(rel), true);
    }
    let summary = &outcome.summary;
    assert!(summary.has_errors(), "run must exit non-zero");
    assert_eq!(summary.probes_attempted, 4);
    assert_eq!(summary.probe_failure_count(), 4);
    assert_eq!(summary.probe_failures.len(), 1);
    assert_eq!(summary.probe_failures[0].samples.len(), 3);

    let rendered = summary.render();
    println!("{rendered}");
    assert!(rendered.contains("Probe failures: 4 of 4"), "{rendered}");
    assert!(
        rendered.contains("Warning: 4 of 4 probes failed. The originals are probably unreadable"),
        "{rendered}"
    );
    assert!(rendered.contains("dimensions and camera metadata"));
}

#[cfg(unix)]
#[test]
fn failed_probe_is_retried_once_the_source_is_readable() {
    let fixture = Fixture::new();
    fixture.write_jpeg_with_exif(
        "2024/05/rotated.jpg",
        Some(6),
        Some("2024:05:01 10:11:12"),
        false,
    );
    let source = fixture.media_path("2024/05/rotated.jpg");
    let before = std::fs::metadata(&source).unwrap();
    if !lock_source(&source) {
        eprintln!("skipping probe retry test: unreadable files stay readable");
        return;
    }

    let first = run_with_config(fixture.config()).unwrap();
    set_readable(&source, true);
    assert_eq!(first.summary.probes_attempted, 1);
    assert_eq!(first.summary.probe_failure_count(), 1);
    let item = shard_item(&fixture, "2024/05/rotated.jpg");
    assert!(item.get("width").is_none_or(Value::is_null), "{item}");

    let after = std::fs::metadata(&source).unwrap();
    assert_eq!(after.len(), before.len());
    assert_eq!(
        fingerprint::mtime_nanos(&after),
        fingerprint::mtime_nanos(&before)
    );

    let second = run_with_config(fixture.config()).unwrap();
    assert!(
        second.summary.errors.is_empty(),
        "{:?}",
        second.summary.errors
    );
    assert_eq!(second.summary.unchanged, 1);
    assert_eq!(second.summary.probes_attempted, 1);
    assert_eq!(second.summary.probed, 1);
    assert!(second.summary.render().contains("Probe failures: 0 of 1"));
    let item = shard_item(&fixture, "2024/05/rotated.jpg");
    assert_eq!(item["width"], 20);
    assert_eq!(item["height"], 40);
    assert_eq!(item["captured_at"], "2024-05-01 10:11:12");
    assert_eq!(item["f_number"], 1.8);
    assert_eq!(item["iso"], 100);

    let third = run_with_config(fixture.config()).unwrap();
    assert!(third.summary.errors.is_empty());
    assert_eq!(third.summary.probes_attempted, 0);
    assert_eq!(third.summary.probed, 0);
    assert!(third.summary.render().contains("Probe failures: 0 of 0"));
}
