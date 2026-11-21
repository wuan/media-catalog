use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Serialize;

use crate::fsutil::write_atomic;
use crate::model::{Catalog, ItemView};
use crate::scan::MediaKind;

pub const JSON_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Serialize)]
struct IndexFile {
    schema_version: u32,
    total_items: usize,
    years: Vec<IndexYear>,
}

#[derive(Debug, Serialize)]
struct IndexYear {
    year: i32,
    count: usize,
    months: Vec<IndexMonth>,
    shard: String,
}

#[derive(Debug, Serialize)]
struct IndexMonth {
    month: u32,
    count: usize,
}

#[derive(Debug, Serialize)]
struct YearShard {
    schema_version: u32,
    year: i32,
    items: Vec<ShardItem>,
}

#[derive(Debug, Serialize)]
struct ShardItem {
    path: String,
    month: u32,
    kind: MediaKind,
    captured_at: String,
    width: Option<u32>,
    height: Option<u32>,
    duration_ms: Option<u64>,
    thumb_url: Option<String>,
    media_url: String,
    thumb_available: bool,
}

impl From<&ItemView> for ShardItem {
    fn from(item: &ItemView) -> Self {
        Self {
            path: item.path.clone(),
            month: item.month,
            kind: item.kind,
            captured_at: item.captured_at.clone(),
            width: item.width,
            height: item.height,
            duration_ms: item.duration_ms,
            thumb_url: item.thumb_url.clone(),
            media_url: item.media_url.clone(),
            thumb_available: item.thumb_available,
        }
    }
}

/// Serialized JSON outputs, kept in memory for deterministic tests.
#[derive(Debug, Clone, Default)]
pub struct JsonOutput {
    pub index: String,
    pub shards: BTreeMap<i32, String>,
}

/// Render the index and one shard per year.
pub fn render_json(catalog: &Catalog) -> Result<JsonOutput> {
    let mut index_years = Vec::with_capacity(catalog.years.len());
    let mut shards = BTreeMap::new();

    for year in &catalog.years {
        let months = year
            .months
            .iter()
            .map(|month| IndexMonth {
                month: month.month,
                count: month.items.len(),
            })
            .collect();
        index_years.push(IndexYear {
            year: year.year,
            count: year.item_count,
            months,
            shard: format!("catalog/{}.json", year.year),
        });

        let items = year
            .months
            .iter()
            .flat_map(|month| month.items.iter())
            .map(ShardItem::from)
            .collect();
        let shard = YearShard {
            schema_version: JSON_SCHEMA_VERSION,
            year: year.year,
            items,
        };
        shards.insert(year.year, to_pretty_json(&shard)?);
    }

    let index = to_pretty_json(&IndexFile {
        schema_version: JSON_SCHEMA_VERSION,
        total_items: catalog.item_count,
        years: index_years,
    })?;

    Ok(JsonOutput { index, shards })
}

fn to_pretty_json<T: Serialize>(value: &T) -> Result<String> {
    let mut text = serde_json::to_string_pretty(value).context("serializing catalog JSON")?;
    text.push('\n');
    Ok(text)
}

/// Write `catalog/index.json` and one shard per year, removing stale shards for
/// years that no longer exist.
pub fn write_json(public_root: &Path, catalog: &Catalog) -> Result<usize> {
    let output = render_json(catalog)?;
    let catalog_dir = public_root.join("catalog");
    let mut written = 0;

    let index_path = catalog_dir.join("index.json");
    write_atomic(&index_path, output.index.as_bytes())?;
    written += 1;

    let mut expected = BTreeSet::new();
    expected.insert(index_path);
    for (year, shard) in &output.shards {
        let path = catalog_dir.join(format!("{year}.json"));
        write_atomic(&path, shard.as_bytes())?;
        expected.insert(path);
        written += 1;
    }

    remove_stale_shards(&catalog_dir, &expected)?;
    Ok(written)
}

fn remove_stale_shards(catalog_dir: &Path, expected: &BTreeSet<PathBuf>) -> Result<()> {
    let entries = match std::fs::read_dir(catalog_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(
                anyhow::Error::new(error).context(format!("reading {}", catalog_dir.display()))
            );
        }
    };
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) == Some("json")
            && !expected.contains(&path)
        {
            std::fs::remove_file(&path)
                .with_context(|| format!("removing stale shard {}", path.display()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{build_catalog, CatalogItemInput};
    use crate::scan::MediaKind;

    fn catalog() -> Catalog {
        build_catalog(
            vec![
                CatalogItemInput {
                    rel_path: "2023/12/a.jpg".into(),
                    year: 2023,
                    month: 12,
                    kind: MediaKind::Image,
                    captured_at: Some("2023-12-01 10:00:00".into()),
                    mtime_ns: 0,
                    width: Some(4000),
                    height: Some(3000),
                    duration_ms: None,
                    thumb_version: Some("10-20".into()),
                    thumb_available: true,
                },
                CatalogItemInput {
                    rel_path: "2024/05/clip.mp4".into(),
                    year: 2024,
                    month: 5,
                    kind: MediaKind::Video,
                    captured_at: Some("2024-05-02 11:00:00".into()),
                    mtime_ns: 0,
                    width: Some(1920),
                    height: Some(1080),
                    duration_ms: Some(12_500),
                    thumb_version: Some("30-40".into()),
                    thumb_available: true,
                },
                CatalogItemInput {
                    rel_path: "2024/05/broken.heic".into(),
                    year: 2024,
                    month: 5,
                    kind: MediaKind::Image,
                    captured_at: Some("2024-05-03 12:00:00".into()),
                    mtime_ns: 0,
                    width: None,
                    height: None,
                    duration_ms: None,
                    thumb_version: None,
                    thumb_available: false,
                },
            ],
            "/media",
            "/thumbs",
        )
    }

    #[test]
    fn renders_index_and_one_shard_per_year() {
        let output = render_json(&catalog()).unwrap();
        assert!(output.index.contains("\"total_items\": 3"));
        assert!(output.index.contains("\"shard\": \"catalog/2024.json\""));
        assert!(output.index.contains("\"shard\": \"catalog/2023.json\""));
        assert_eq!(output.shards.len(), 2);
        assert!(output.shards[&2024].contains("\"kind\": \"video\""));
        assert!(output.shards[&2024].contains("\"duration_ms\": 12500"));
        assert!(output.shards[&2024].contains("\"thumb_available\": false"));
    }

    #[test]
    fn output_is_byte_identical_across_builds() {
        let first = render_json(&catalog()).unwrap();
        let second = render_json(&catalog()).unwrap();
        assert_eq!(first.index, second.index);
        assert_eq!(first.shards, second.shards);
    }

    #[test]
    fn no_private_metadata_fields_are_serialized() {
        let output = render_json(&catalog()).unwrap();
        let all = format!(
            "{}{}",
            output.index,
            output.shards.values().cloned().collect::<String>()
        );
        for forbidden in ["gps", "camera", "make", "model", "latitude", "longitude"] {
            assert!(!all.to_ascii_lowercase().contains(forbidden));
        }
    }

    #[test]
    fn snapshot_index_and_shard() {
        let output = render_json(&catalog()).unwrap();
        insta::assert_snapshot!("index_json", output.index);
        insta::assert_snapshot!("shard_2024_json", output.shards[&2024]);
    }

    #[test]
    fn removes_stale_shards_for_vanished_years() {
        let temp = tempfile::tempdir().unwrap();
        let catalog_dir = temp.path().join("catalog");
        std::fs::create_dir_all(&catalog_dir).unwrap();
        std::fs::write(catalog_dir.join("2019.json"), "{}").unwrap();
        write_json(temp.path(), &catalog()).unwrap();
        assert!(!catalog_dir.join("2019.json").exists());
        assert!(catalog_dir.join("2024.json").exists());
        assert!(catalog_dir.join("index.json").exists());
    }
}
