use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::fsutil::write_atomic;
use crate::model::{month_label, month_short_label, Catalog, ItemView, MonthView, YearView};
use crate::paths::item_name;
use crate::scan::MediaKind;

pub const JSON_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexFile {
    pub schema_version: u32,
    pub total_items: usize,
    pub years: Vec<IndexYear>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexYear {
    pub year: i32,
    pub count: usize,
    pub months: Vec<IndexMonth>,
    pub shard: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexMonth {
    pub month: u32,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YearShard {
    pub schema_version: u32,
    pub year: i32,
    pub items: Vec<ShardItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardItem {
    pub path: String,
    pub month: u32,
    pub kind: MediaKind,
    pub captured_at: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration_ms: Option<u64>,
    pub thumb_url: Option<String>,
    pub media_url: String,
    pub thumb_available: bool,
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

impl ShardItem {
    pub fn to_item_view(&self) -> ItemView {
        ItemView {
            path: self.path.clone(),
            name: item_name(&self.path),
            month: self.month,
            kind: self.kind,
            captured_at: self.captured_at.clone(),
            width: self.width,
            height: self.height,
            duration_ms: self.duration_ms,
            thumb_url: self.thumb_url.clone(),
            media_url: self.media_url.clone(),
            thumb_available: self.thumb_available,
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
            shard: shard_path(year.year),
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

/// Index-relative shard path for a year.
pub fn shard_path(year: i32) -> String {
    format!("catalog/{year}.json")
}

/// Validate and normalize the shard path recorded in the index.
pub fn shard_file_name(year: i32, shard: &str) -> Result<String> {
    let expected = format!("{year}.json");
    let expected_path = format!("catalog/{expected}");
    if shard != expected_path {
        bail!(
            "catalog index for {year} references unsupported shard path {shard:?}; expected {expected_path:?}"
        );
    }
    Ok(expected)
}

/// Parse an index document.
pub fn parse_index(text: &str) -> Result<IndexFile> {
    serde_json::from_str(text).context("parsing catalog index")
}

/// Parse a year shard document.
pub fn parse_shard(text: &str) -> Result<YearShard> {
    serde_json::from_str(text).context("parsing catalog shard")
}

/// Rebuild the shared catalog model from a parsed index and shards.
pub fn catalog_from_parts(index: &IndexFile, shards: &BTreeMap<i32, YearShard>) -> Result<Catalog> {
    if index.schema_version != JSON_SCHEMA_VERSION {
        bail!(
            "unsupported catalog schema version {} (expected {})",
            index.schema_version,
            JSON_SCHEMA_VERSION
        );
    }

    let declared_total: usize = index.years.iter().map(|year| year.count).sum();
    if declared_total != index.total_items {
        bail!(
            "catalog index total_items {} does not match year counts {}",
            index.total_items,
            declared_total
        );
    }

    let mut years = Vec::with_capacity(index.years.len());
    let mut total = 0usize;

    for index_year in &index.years {
        let shard = shards
            .get(&index_year.year)
            .with_context(|| format!("missing shard for year {}", index_year.year))?;
        if shard.schema_version != JSON_SCHEMA_VERSION {
            bail!(
                "shard {} has unsupported schema version {}",
                index_year.year,
                shard.schema_version
            );
        }
        if shard.year != index_year.year {
            bail!("shard for {} declares year {}", index_year.year, shard.year);
        }
        if shard.items.len() != index_year.count {
            bail!(
                "shard {} has {} items but the index declares {}",
                index_year.year,
                shard.items.len(),
                index_year.count
            );
        }

        let mut months: Vec<MonthView> = Vec::new();
        for item in &shard.items {
            if !(1..=12).contains(&item.month) {
                bail!(
                    "shard {} item {:?} has invalid month {}",
                    index_year.year,
                    item.path,
                    item.month
                );
            }
            let view = item.to_item_view();
            match months.last_mut() {
                Some(month) if month.month == view.month => month.items.push(view),
                _ => months.push(MonthView {
                    year: index_year.year,
                    month: view.month,
                    label: month_label(index_year.year, view.month),
                    short_label: month_short_label(index_year.year, view.month),
                    items: vec![view],
                }),
            }
        }

        if months.len() != index_year.months.len() {
            bail!(
                "shard {} has {} month groups but the index declares {}",
                index_year.year,
                months.len(),
                index_year.months.len()
            );
        }
        for index_month in &index_year.months {
            let actual = months
                .iter()
                .find(|month| month.month == index_month.month)
                .map(|month| month.items.len())
                .unwrap_or(0);
            if actual != index_month.count {
                bail!(
                    "shard {} month {} has {} items but the index declares {}",
                    index_year.year,
                    index_month.month,
                    actual,
                    index_month.count
                );
            }
        }

        total += shard.items.len();
        years.push(YearView {
            year: index_year.year,
            months,
            item_count: shard.items.len(),
        });
    }

    if total != index.total_items {
        bail!(
            "catalog shards contain {} items but the index declares {}",
            total,
            index.total_items
        );
    }

    Ok(Catalog {
        years,
        item_count: total,
    })
}

/// Parse an index plus shard documents into the shared catalog model.
pub fn catalog_from_json(index_text: &str, shard_texts: &BTreeMap<i32, String>) -> Result<Catalog> {
    let index = parse_index(index_text)?;
    let mut shards = BTreeMap::new();
    for index_year in &index.years {
        let text = shard_texts
            .get(&index_year.year)
            .with_context(|| format!("missing shard text for year {}", index_year.year))?;
        shards.insert(index_year.year, parse_shard(text)?);
    }
    catalog_from_parts(&index, &shards)
}

/// Read `catalog/index.json` and its referenced shards from a catalog directory.
pub fn load_catalog(catalog_dir: &Path) -> Result<Catalog> {
    let index_text = std::fs::read_to_string(catalog_dir.join("index.json"))
        .with_context(|| format!("reading {}", catalog_dir.join("index.json").display()))?;
    let index = parse_index(&index_text)?;

    let mut shards = BTreeMap::new();
    for index_year in &index.years {
        let name = shard_file_name(index_year.year, &index_year.shard)?;
        let path = catalog_dir.join(name);
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        shards.insert(index_year.year, parse_shard(&text)?);
    }

    catalog_from_parts(&index, &shards)
}

/// Write one shard per year first, then `catalog/index.json` as the commit
/// point, then remove stale shards for years no longer referenced.
pub fn write_json(data_root: &Path, catalog: &Catalog) -> Result<usize> {
    let output = render_json(catalog)?;
    let catalog_dir = data_root.join("catalog");
    let mut written = 0;
    let mut expected = BTreeSet::new();

    for (year, shard) in &output.shards {
        let path = catalog_dir.join(format!("{year}.json"));
        write_atomic(&path, shard.as_bytes())?;
        expected.insert(path);
        written += 1;
    }

    let index_path = catalog_dir.join("index.json");
    write_atomic(&index_path, output.index.as_bytes())?;
    expected.insert(index_path);
    written += 1;

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
    fn round_trips_json_into_the_shared_catalog_model() {
        let original = catalog();
        let output = render_json(&original).unwrap();
        let parsed = catalog_from_json(&output.index, &output.shards).unwrap();
        assert_eq!(parsed.item_count, original.item_count);
        assert_eq!(parsed.years.len(), original.years.len());
        for (left, right) in parsed.years.iter().zip(&original.years) {
            assert_eq!(left.year, right.year);
            assert_eq!(left.item_count, right.item_count);
            assert_eq!(left.months.len(), right.months.len());
            for (lmonth, rmonth) in left.months.iter().zip(&right.months) {
                assert_eq!(lmonth.month, rmonth.month);
                assert_eq!(lmonth.items, rmonth.items);
            }
        }

        let rerendered = render_json(&parsed).unwrap();
        assert_eq!(rerendered.index, output.index);
        assert_eq!(rerendered.shards, output.shards);
    }

    #[test]
    fn rejects_index_counts_that_disagree_with_shards() {
        let output = render_json(&catalog()).unwrap();
        let mut index: IndexFile = parse_index(&output.index).unwrap();
        index.years[0].count += 1;
        let mut shards = BTreeMap::new();
        for year in &index.years {
            shards.insert(year.year, parse_shard(&output.shards[&year.year]).unwrap());
        }
        assert!(catalog_from_parts(&index, &shards).is_err());
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

    #[test]
    fn writes_shards_before_the_index_and_keeps_the_old_index_on_failure() {
        let temp = tempfile::tempdir().unwrap();
        let catalog_dir = temp.path().join("catalog");
        std::fs::create_dir_all(&catalog_dir).unwrap();
        std::fs::write(catalog_dir.join("index.json"), "previous-index").unwrap();

        // A directory at a shard destination makes that shard write fail.
        std::fs::create_dir(catalog_dir.join("2024.json")).unwrap();
        let error = write_json(temp.path(), &catalog()).unwrap_err();
        assert!(error.to_string().contains("2024.json"));
        assert_eq!(
            std::fs::read_to_string(catalog_dir.join("index.json")).unwrap(),
            "previous-index"
        );
    }
}
