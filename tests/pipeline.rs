mod common;

use std::collections::HashSet;
use std::time::Duration;

use common::{set_readable, Fixture};
use media_catalog::lock::BuildLock;
use media_catalog::state::StateDb;
use media_catalog::{json, render, run_with_config};
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
    for forbidden in [
        "gps",
        "latitude",
        "longitude",
        "camera",
        "\"make\"",
        "\"model\"",
    ] {
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
