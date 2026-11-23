use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::events;
use crate::fsutil::write_atomic;
use crate::model::{
    month_label, month_short_label, Catalog, EventMeta, EventSection, EventView, ItemView,
    MonthView, SectionView, YearView,
};
use crate::paths::item_name;
use crate::probe::Exposure;
use crate::scan::MediaKind;

pub const JSON_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexFile {
    pub schema_version: u32,
    pub total_items: usize,
    pub years: Vec<IndexYear>,
    /// Authored events. Omitted for catalogs without events.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<IndexEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEvent {
    pub slug: String,
    pub title: String,
    pub start: String,
    pub end: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Resolved section ranges. Omitted for month-grouped events.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sections: Vec<IndexSection>,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexSection {
    pub slug: String,
    pub title: String,
    pub start: String,
    pub end: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl From<&EventSection> for IndexSection {
    fn from(section: &EventSection) -> Self {
        Self {
            slug: section.slug.clone(),
            title: section.title.clone(),
            start: section.start.clone(),
            end: section.end.clone(),
            description: section.description.clone(),
        }
    }
}

impl From<&SectionView> for IndexSection {
    fn from(section: &SectionView) -> Self {
        Self {
            slug: section.slug.clone(),
            title: section.title.clone(),
            start: section.start.clone(),
            end: section.end.clone(),
            description: section.description.clone(),
        }
    }
}

impl From<&EventView> for IndexEvent {
    fn from(event: &EventView) -> Self {
        Self {
            slug: event.slug.clone(),
            title: event.title.clone(),
            start: event.start.clone(),
            end: event.end.clone(),
            description: event.description.clone(),
            sections: event.sections.iter().map(IndexSection::from).collect(),
            count: event.item_count,
        }
    }
}

impl IndexEvent {
    fn to_meta(&self) -> EventMeta {
        EventMeta {
            slug: self.slug.clone(),
            title: self.title.clone(),
            start: self.start.clone(),
            end: self.end.clone(),
            description: self.description.clone(),
            item_count: self.count,
            sections: self
                .sections
                .iter()
                .map(|section| EventSection {
                    slug: section.slug.clone(),
                    title: section.title.clone(),
                    start: section.start.clone(),
                    end: section.end.clone(),
                    description: section.description.clone(),
                })
                .collect(),
        }
    }
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Original file size in bytes. Absent in shards published before sizes
    /// were recorded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub f_number: Option<f32>,
    /// Reduced exposure fraction such as `1/125`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exposure_time: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub iso: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focal_length_mm: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latitude: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub longitude: Option<f64>,
    pub thumb_url: Option<String>,
    pub media_url: String,
    pub thumb_available: bool,
    /// Event slugs this item belongs to. Omitted for catalogs without events.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<String>,
}

impl From<&ItemView> for ShardItem {
    fn from(item: &ItemView) -> Self {
        Self {
            path: item.path.clone(),
            month: item.month,
            kind: item.kind,
            captured_at: item.captured_at.clone(),
            model: item.model.clone(),
            size_bytes: item.size,
            width: item.width,
            height: item.height,
            duration_ms: item.duration_ms,
            f_number: item.f_number,
            exposure_time: item.exposure.map(|exposure| exposure.fraction()),
            iso: item.iso,
            focal_length_mm: item.focal_length_mm,
            latitude: item.latitude,
            longitude: item.longitude,
            thumb_url: item.thumb_url.clone(),
            media_url: item.media_url.clone(),
            thumb_available: item.thumb_available,
            events: item.events.clone(),
        }
    }
}

impl ShardItem {
    pub fn to_item_view(&self, year: i32) -> ItemView {
        ItemView {
            path: self.path.clone(),
            name: item_name(&self.path),
            year,
            month: self.month,
            kind: self.kind,
            captured_at: self.captured_at.clone(),
            model: self.model.clone(),
            size: self.size_bytes,
            width: self.width,
            height: self.height,
            duration_ms: self.duration_ms,
            f_number: self.f_number,
            exposure: self.exposure_time.as_deref().and_then(Exposure::parse),
            iso: self.iso,
            focal_length_mm: self.focal_length_mm,
            latitude: self.latitude,
            longitude: self.longitude,
            thumb_url: self.thumb_url.clone(),
            media_url: self.media_url.clone(),
            thumb_available: self.thumb_available,
            events: self.events.clone(),
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
        events: catalog.events.iter().map(IndexEvent::from).collect(),
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
            let view = item.to_item_view(index_year.year);
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

    let mut catalog = Catalog {
        years,
        item_count: total,
        events: Vec::new(),
    };

    let definitions = index
        .events
        .iter()
        .map(IndexEvent::to_meta)
        .collect::<Vec<_>>();
    catalog.events = events::build_event_views(&catalog, &definitions)?;

    Ok(catalog)
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
                    model: None,
                    mtime_ns: 0,
                    size: 3_400_000,
                    width: Some(4000),
                    height: Some(3000),
                    duration_ms: None,
                    thumb_version: Some("10-20".into()),
                    thumb_available: true,
                    ..CatalogItemInput::default()
                },
                CatalogItemInput {
                    rel_path: "2024/05/clip.mp4".into(),
                    year: 2024,
                    month: 5,
                    kind: MediaKind::Video,
                    captured_at: Some("2024-05-02 11:00:00".into()),
                    model: Some("iPhone 15 Pro".into()),
                    mtime_ns: 0,
                    size: 48_000_000,
                    width: Some(1920),
                    height: Some(1080),
                    duration_ms: Some(12_500),
                    thumb_version: Some("30-40".into()),
                    thumb_available: true,
                    ..CatalogItemInput::default()
                },
                CatalogItemInput {
                    rel_path: "2024/05/broken.heic".into(),
                    year: 2024,
                    month: 5,
                    kind: MediaKind::Image,
                    captured_at: Some("2024-05-03 12:00:00".into()),
                    model: None,
                    mtime_ns: 0,
                    size: 532_000,
                    width: None,
                    height: None,
                    duration_ms: None,
                    thumb_version: None,
                    thumb_available: false,
                    ..CatalogItemInput::default()
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
        assert!(output.shards[&2024].contains("\"model\": \"iPhone 15 Pro\""));
        assert!(output.shards[&2024].contains("\"thumb_available\": false"));
    }

    #[test]
    fn shards_publish_file_sizes_in_bytes() {
        let output = render_json(&catalog()).unwrap();
        assert!(output.shards[&2023].contains("\"size_bytes\": 3400000"));
        assert!(output.shards[&2024].contains("\"size_bytes\": 48000000"));

        let parsed = catalog_from_json(&output.index, &output.shards).unwrap();
        let clip = parsed
            .years
            .iter()
            .flat_map(|year| &year.months)
            .flat_map(|month| &month.items)
            .find(|item| item.path == "2024/05/clip.mp4")
            .unwrap();
        assert_eq!(clip.size, Some(48_000_000));
    }

    #[test]
    fn shard_items_without_size_bytes_load_with_unknown_size() {
        let output = render_json(&catalog()).unwrap();
        let shards = output
            .shards
            .iter()
            .map(|(year, text)| {
                let stripped = text
                    .lines()
                    .filter(|line| !line.contains("\"size_bytes\""))
                    .collect::<Vec<_>>()
                    .join("\n");
                (*year, stripped)
            })
            .collect();
        let parsed = catalog_from_json(&output.index, &shards).unwrap();
        for year in &parsed.years {
            for month in &year.months {
                for item in &month.items {
                    assert_eq!(item.size, None, "{}", item.path);
                }
            }
        }
    }

    #[test]
    fn output_is_byte_identical_across_builds() {
        let first = render_json(&catalog()).unwrap();
        let second = render_json(&catalog()).unwrap();
        assert_eq!(first.index, second.index);
        assert_eq!(first.shards, second.shards);
    }

    #[test]
    fn location_fields_are_absent_when_not_published() {
        let output = render_json(&catalog()).unwrap();
        let all = format!(
            "{}{}",
            output.index,
            output.shards.values().cloned().collect::<String>()
        );
        for forbidden in ["gps", "location", "latitude", "longitude", "serial"] {
            assert!(!all.to_ascii_lowercase().contains(forbidden));
        }
    }

    #[test]
    fn publishes_camera_settings_and_coordinates() {
        let mut item = CatalogItemInput {
            rel_path: "2024/05/a.jpg".into(),
            year: 2024,
            month: 5,
            kind: MediaKind::Image,
            captured_at: Some("2024-05-01 10:00:00".into()),
            f_number: Some(1.8),
            exposure: crate::probe::Exposure::new(1, 125),
            iso: Some(100),
            focal_length_mm: Some(26.0),
            latitude: Some(52.520008),
            longitude: Some(13.404954),
            ..CatalogItemInput::default()
        };
        item.size = 3_400_000;
        let catalog = build_catalog(vec![item], "/media", "/thumbs");
        let output = render_json(&catalog).unwrap();
        let shard = &output.shards[&2024];
        assert!(shard.contains("\"f_number\": 1.8"));
        assert!(shard.contains("\"exposure_time\": \"1/125\""));
        assert!(shard.contains("\"iso\": 100"));
        assert!(shard.contains("\"focal_length_mm\": 26.0"));
        assert!(shard.contains("\"latitude\": 52.520008"));
        assert!(shard.contains("\"longitude\": 13.404954"));

        let parsed = catalog_from_json(&output.index, &output.shards).unwrap();
        let parsed_item = &parsed.years[0].months[0].items[0];
        assert_eq!(parsed_item.f_number, Some(1.8));
        assert_eq!(parsed_item.exposure, crate::probe::Exposure::new(1, 125));
        assert_eq!(parsed_item.iso, Some(100));
        assert_eq!(parsed_item.focal_length_mm, Some(26.0));
        assert_eq!(parsed_item.latitude, Some(52.520008));
        assert_eq!(parsed_item.longitude, Some(13.404954));
    }

    #[test]
    fn omits_unknown_camera_settings_and_location() {
        let output = render_json(&catalog()).unwrap();
        let all = output.shards.values().cloned().collect::<String>();
        for absent in [
            "\"f_number\"",
            "\"exposure_time\"",
            "\"iso\"",
            "\"focal_length_mm\"",
            "\"latitude\"",
            "\"longitude\"",
        ] {
            assert!(!all.contains(absent), "unexpected {absent}");
        }
    }

    #[test]
    fn snapshot_index_and_shard() {
        let output = render_json(&catalog()).unwrap();
        insta::assert_snapshot!("index_json", output.index);
        insta::assert_snapshot!("shard_2024_json", output.shards[&2024]);
    }

    fn event_catalog() -> Catalog {
        let mut catalog = build_catalog(
            vec![
                CatalogItemInput {
                    rel_path: "2024/05/a.jpg".into(),
                    year: 2024,
                    month: 5,
                    kind: MediaKind::Image,
                    captured_at: Some("2024-05-30 10:11:12".into()),
                    model: None,
                    mtime_ns: 0,
                    size: 0,
                    width: Some(4000),
                    height: Some(3000),
                    duration_ms: None,
                    thumb_version: Some("10-20".into()),
                    thumb_available: true,
                    ..CatalogItemInput::default()
                },
                CatalogItemInput {
                    rel_path: "2024/06/b.jpg".into(),
                    year: 2024,
                    month: 6,
                    kind: MediaKind::Image,
                    captured_at: Some("2024-06-02 09:00:00".into()),
                    model: None,
                    mtime_ns: 0,
                    size: 0,
                    width: Some(4000),
                    height: Some(3000),
                    duration_ms: None,
                    thumb_version: Some("11-21".into()),
                    thumb_available: true,
                    ..CatalogItemInput::default()
                },
                CatalogItemInput {
                    rel_path: "2024/06/outside.jpg".into(),
                    year: 2024,
                    month: 6,
                    kind: MediaKind::Image,
                    captured_at: Some("2024-06-20 09:00:00".into()),
                    model: None,
                    mtime_ns: 0,
                    size: 0,
                    width: Some(4000),
                    height: Some(3000),
                    duration_ms: None,
                    thumb_version: Some("12-22".into()),
                    thumb_available: true,
                    ..CatalogItemInput::default()
                },
            ],
            "/media",
            "/thumbs",
        );
        crate::events::attach_events(
            &mut catalog,
            vec![
                EventMeta {
                    slug: "may-trip".into(),
                    title: "May trip".into(),
                    start: "2024-05-28 09:00".into(),
                    end: "2024-06-05 18:30".into(),
                    description: Some("A crossing event.".into()),
                    item_count: 0,
                    sections: Vec::new(),
                },
                EventMeta {
                    slug: "june".into(),
                    title: "June".into(),
                    start: "2024-06-01 00:00".into(),
                    end: "2024-06-30 23:59".into(),
                    description: None,
                    item_count: 0,
                    sections: Vec::new(),
                },
            ],
        )
        .unwrap();
        catalog
    }

    #[test]
    fn snapshot_index_and_shard_with_events() {
        let output = render_json(&event_catalog()).unwrap();
        insta::assert_snapshot!("index_events_json", output.index);
        insta::assert_snapshot!("shard_2024_events_json", output.shards[&2024]);
    }

    #[test]
    fn catalogs_without_events_omit_event_fields() {
        let output = render_json(&catalog()).unwrap();
        assert!(!output.index.contains("\"events\""));
        for shard in output.shards.values() {
            assert!(!shard.contains("\"events\""));
        }
    }

    #[test]
    fn event_output_is_byte_identical_across_builds() {
        let first = render_json(&event_catalog()).unwrap();
        let second = render_json(&event_catalog()).unwrap();
        assert_eq!(first.index, second.index);
        assert_eq!(first.shards, second.shards);
    }

    fn section_event_catalog() -> Catalog {
        let mut catalog = build_catalog(
            vec![
                CatalogItemInput {
                    rel_path: "2024/05/lisbon.jpg".into(),
                    year: 2024,
                    month: 5,
                    kind: MediaKind::Image,
                    captured_at: Some("2024-05-02 10:00:00".into()),
                    model: None,
                    mtime_ns: 0,
                    size: 0,
                    width: Some(4000),
                    height: Some(3000),
                    duration_ms: None,
                    thumb_version: Some("10-20".into()),
                    thumb_available: true,
                    ..CatalogItemInput::default()
                },
                CatalogItemInput {
                    rel_path: "2024/05/algarve.jpg".into(),
                    year: 2024,
                    month: 5,
                    kind: MediaKind::Image,
                    captured_at: Some("2024-05-10 10:00:00".into()),
                    model: None,
                    mtime_ns: 0,
                    size: 0,
                    width: Some(4000),
                    height: Some(3000),
                    duration_ms: None,
                    thumb_version: Some("11-21".into()),
                    thumb_available: true,
                    ..CatalogItemInput::default()
                },
            ],
            "/media",
            "/thumbs",
        );
        crate::events::attach_events(
            &mut catalog,
            vec![EventMeta {
                slug: "portugal".into(),
                title: "Portugal".into(),
                start: "2024-05-01 00:00".into(),
                end: "2024-05-31 23:59".into(),
                description: None,
                item_count: 0,
                sections: vec![
                    EventSection {
                        slug: "lisbon".into(),
                        title: "Lisbon".into(),
                        start: "2024-05-01 00:00".into(),
                        end: "2024-05-09 23:59".into(),
                        description: Some("Capital".into()),
                    },
                    EventSection {
                        slug: "algarve".into(),
                        title: "Algarve".into(),
                        start: "2024-05-10 00:00".into(),
                        end: "2024-05-31 23:59".into(),
                        description: None,
                    },
                ],
            }],
        )
        .unwrap();
        catalog
    }

    #[test]
    fn publishes_and_round_trips_section_ranges() {
        let original = section_event_catalog();
        let output = render_json(&original).unwrap();
        assert!(output.index.contains("\"slug\": \"lisbon\""));
        assert!(output.index.contains("\"end\": \"2024-05-09 23:59\""));
        insta::assert_snapshot!("index_sections_json", output.index);

        let parsed = catalog_from_json(&output.index, &output.shards).unwrap();
        let event = parsed.event("portugal").unwrap();
        assert_eq!(event.sections.len(), 2);
        assert_eq!(event.sections[0].slug, "lisbon");
        assert_eq!(event.sections[0].items[0].path, "2024/05/lisbon.jpg");
        assert_eq!(event.sections[1].items[0].path, "2024/05/algarve.jpg");
        assert!(event.other_items.is_empty());

        let rerendered = render_json(&parsed).unwrap();
        assert_eq!(rerendered.index, output.index);
        assert_eq!(rerendered.shards, output.shards);
    }

    #[test]
    fn readme_json_example_matches_generated_sections() {
        let readme = include_str!("../README.md");
        let json_section = &readme[readme
            .find("## JSON catalog and search")
            .expect("README JSON section")..];
        let json_start = json_section.find("```json").expect("README JSON block") + 7;
        let json_end = json_start + json_section[json_start..].find("```").expect("block end");
        let index = parse_index(&json_section[json_start..json_end]).unwrap();

        let toml_start = readme.find("[[events]]").expect("README event example");
        let toml_end = toml_start + readme[toml_start..].find("```").expect("block end");
        let events =
            crate::events::parse_events(&readme[toml_start..toml_end], "README.md").unwrap();

        let published = index.events[0]
            .sections
            .iter()
            .map(|section| EventSection {
                slug: section.slug.clone(),
                title: section.title.clone(),
                start: section.start.clone(),
                end: section.end.clone(),
                description: section.description.clone(),
            })
            .collect::<Vec<_>>();
        assert_eq!(published, events[0].sections);
    }

    #[test]
    fn rejects_invalid_published_sections() {
        let output = render_json(&section_event_catalog()).unwrap();
        let mut index = parse_index(&output.index).unwrap();
        let shards = index
            .years
            .iter()
            .map(|year| (year.year, parse_shard(&output.shards[&year.year]).unwrap()))
            .collect::<BTreeMap<_, _>>();

        index.events[0].sections[1].start = "2024-05-09 23:59".into();
        index.events[0].sections[1].end = "2024-05-31 23:59".into();
        let error = catalog_from_parts(&index, &shards).unwrap_err();
        assert!(error.to_string().contains("overlaps or is out of order"));

        let mut outside = parse_index(&output.index).unwrap();
        outside.events[0].sections[0].start = "2024-04-30 00:00".into();
        let error = catalog_from_parts(&outside, &shards).unwrap_err();
        assert!(error.to_string().contains("outside"));

        let mut duplicate = parse_index(&output.index).unwrap();
        duplicate.events[0].sections[1].slug = "lisbon".into();
        let error = catalog_from_parts(&duplicate, &shards).unwrap_err();
        assert!(error.to_string().contains("duplicate section slug"));
    }

    #[test]
    fn round_trips_event_metadata_and_membership() {
        let original = event_catalog();
        let output = render_json(&original).unwrap();
        let parsed = catalog_from_json(&output.index, &output.shards).unwrap();

        assert_eq!(parsed.events.len(), original.events.len());
        for (left, right) in parsed.events.iter().zip(&original.events) {
            assert_eq!(left.slug, right.slug);
            assert_eq!(left.title, right.title);
            assert_eq!(left.item_count, right.item_count);
            assert_eq!(left.months.len(), right.months.len());
            for (lmonth, rmonth) in left.months.iter().zip(&right.months) {
                assert_eq!(lmonth.year, rmonth.year);
                assert_eq!(lmonth.month, rmonth.month);
                assert_eq!(lmonth.items, rmonth.items);
            }
        }

        let rerendered = render_json(&parsed).unwrap();
        assert_eq!(rerendered.index, output.index);
        assert_eq!(rerendered.shards, output.shards);
    }

    #[test]
    fn catalogs_without_event_fields_load_as_having_no_events() {
        let output = render_json(&catalog()).unwrap();
        let parsed = catalog_from_json(&output.index, &output.shards).unwrap();
        assert!(parsed.events.is_empty());
        for year in &parsed.years {
            for month in &year.months {
                for item in &month.items {
                    assert!(item.events.is_empty());
                }
            }
        }
    }

    #[test]
    fn rejects_undeclared_event_slugs_and_count_mismatches() {
        let output = render_json(&event_catalog()).unwrap();
        let index = parse_index(&output.index).unwrap();
        let shards = index
            .years
            .iter()
            .map(|year| (year.year, parse_shard(&output.shards[&year.year]).unwrap()))
            .collect::<BTreeMap<_, _>>();

        let mut undeclared_shards = shards.clone();
        undeclared_shards
            .get_mut(&2024)
            .unwrap()
            .items
            .first_mut()
            .unwrap()
            .events
            .push("ghost".to_string());
        let error = catalog_from_parts(&index, &undeclared_shards).unwrap_err();
        assert!(error.to_string().contains("undeclared event slug"));

        let mut wrong_count = index.clone();
        wrong_count.events[0].count += 1;
        let error = catalog_from_parts(&wrong_count, &shards).unwrap_err();
        assert!(error.to_string().contains("declares"));

        let mut duplicate = index.clone();
        duplicate.events.push(duplicate.events[0].clone());
        let error = catalog_from_parts(&duplicate, &shards).unwrap_err();
        assert!(error.to_string().contains("duplicate event slug"));
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
