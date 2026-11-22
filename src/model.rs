use std::collections::BTreeMap;

use chrono::NaiveDate;
use serde::Serialize;

use crate::paths::{item_name, thumb_url, url_for};
use crate::probe::timestamp_from_nanos;
use crate::scan::MediaKind;

/// Input row used to build the rendered catalog model.
#[derive(Debug, Clone)]
pub struct CatalogItemInput {
    pub rel_path: String,
    pub year: i32,
    pub month: u32,
    pub kind: MediaKind,
    pub captured_at: Option<String>,
    pub mtime_ns: i64,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration_ms: Option<u64>,
    pub thumb_version: Option<String>,
    pub thumb_available: bool,
}

/// A single item as exposed to the renderer and JSON writer.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ItemView {
    pub path: String,
    pub name: String,
    /// Folder year. Kept in the model so event month groups can address
    /// `/YYYY/MM/` pages even though shard records only store the month.
    #[serde(skip)]
    pub year: i32,
    pub month: u32,
    pub kind: MediaKind,
    pub captured_at: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration_ms: Option<u64>,
    pub thumb_url: Option<String>,
    pub media_url: String,
    pub thumb_available: bool,
    /// Event slugs this item belongs to, sorted independently of config order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<String>,
}

/// An event section resolved to an inclusive published range.
///
/// Authors declare a bookmark timestamp; the generator publishes concrete
/// `start`/`end` ranges so the server never interprets bookmark semantics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventSection {
    pub slug: String,
    pub title: String,
    pub start: String,
    pub end: String,
    pub description: Option<String>,
}

/// Authored event metadata shared by the generator and the server.
///
/// `start` and `end` are canonical `YYYY-MM-DD HH:MM` strings, so lexical order
/// matches chronological order. `item_count` is filled from membership by the
/// generator and validated against the shards by the server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventMeta {
    pub slug: String,
    pub title: String,
    pub start: String,
    pub end: String,
    pub description: Option<String>,
    pub item_count: usize,
    pub sections: Vec<EventSection>,
}

/// A published section with the items whose timestamps fall inside its range.
#[derive(Debug, Clone)]
pub struct SectionView {
    pub slug: String,
    pub title: String,
    pub start: String,
    pub end: String,
    pub description: Option<String>,
    pub items: Vec<ItemView>,
}

/// One event and its matching items grouped by folder month.
#[derive(Debug, Clone)]
pub struct EventView {
    pub slug: String,
    pub title: String,
    pub start: String,
    pub end: String,
    pub description: Option<String>,
    pub item_count: usize,
    pub months: Vec<MonthView>,
    /// Published sections in chronological order; empty for month-grouped events.
    pub sections: Vec<SectionView>,
    /// Event items that fall outside every published section.
    pub other_items: Vec<ItemView>,
}

/// One month of items.
#[derive(Debug, Clone)]
pub struct MonthView {
    pub year: i32,
    pub month: u32,
    pub label: String,
    pub short_label: String,
    pub items: Vec<ItemView>,
}

/// One year of months.
#[derive(Debug, Clone)]
pub struct YearView {
    pub year: i32,
    pub months: Vec<MonthView>,
    pub item_count: usize,
}

/// Deterministically ordered catalog.
#[derive(Debug, Clone, Default)]
pub struct Catalog {
    pub years: Vec<YearView>,
    pub item_count: usize,
    /// Events ordered by newest start and then slug.
    pub events: Vec<EventView>,
}

impl Catalog {
    pub fn year(&self, year: i32) -> Option<&YearView> {
        self.years.iter().find(|view| view.year == year)
    }

    pub fn month(&self, year: i32, month: u32) -> Option<&MonthView> {
        self.year(year)?
            .months
            .iter()
            .find(|view| view.month == month)
    }

    pub fn event(&self, slug: &str) -> Option<&EventView> {
        self.events.iter().find(|view| view.slug == slug)
    }
}

/// Resolve a capture timestamp, falling back to the file modification time.
pub fn capture_timestamp(captured_at: Option<String>, mtime_ns: i64) -> String {
    captured_at
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| timestamp_from_nanos(mtime_ns))
}

pub fn month_label(year: i32, month: u32) -> String {
    NaiveDate::from_ymd_opt(year, month, 1)
        .map(|date| date.format("%B %Y").to_string())
        .unwrap_or_else(|| format!("{year}-{month:02}"))
}

pub fn month_short_label(year: i32, month: u32) -> String {
    NaiveDate::from_ymd_opt(year, month, 1)
        .map(|date| date.format("%B").to_string())
        .unwrap_or_else(|| format!("{month:02}"))
}

/// Build the deterministic model: newest years/months first, and items ordered by
/// capture timestamp descending with the relative path as a stable tie-break.
pub fn build_catalog(
    items: Vec<CatalogItemInput>,
    media_url_prefix: &str,
    thumb_url_prefix: &str,
) -> Catalog {
    let mut by_year: BTreeMap<i32, BTreeMap<u32, Vec<ItemView>>> = BTreeMap::new();

    for item in items {
        let available = item.thumb_available && item.thumb_version.is_some();
        let view = ItemView {
            name: item_name(&item.rel_path),
            media_url: url_for(media_url_prefix, &item.rel_path),
            thumb_url: item
                .thumb_version
                .as_deref()
                .filter(|_| available)
                .map(|version| thumb_url(thumb_url_prefix, &item.rel_path, version)),
            captured_at: capture_timestamp(item.captured_at, item.mtime_ns)
                .trim()
                .to_string(),
            path: item.rel_path,
            year: item.year,
            month: item.month,
            kind: item.kind,
            width: item.width,
            height: item.height,
            duration_ms: item.duration_ms,
            thumb_available: available,
            events: Vec::new(),
        };
        by_year
            .entry(item.year)
            .or_default()
            .entry(item.month)
            .or_default()
            .push(view);
    }

    let mut years = Vec::with_capacity(by_year.len());
    let mut total = 0usize;
    for (year, months) in by_year.into_iter().rev() {
        let mut month_views = Vec::with_capacity(months.len());
        for (month, mut items) in months.into_iter().rev() {
            items.sort_by(|left, right| {
                right
                    .captured_at
                    .cmp(&left.captured_at)
                    .then_with(|| left.path.cmp(&right.path))
            });
            total += items.len();
            month_views.push(MonthView {
                year,
                month,
                label: month_label(year, month),
                short_label: month_short_label(year, month),
                items,
            });
        }
        let item_count = month_views.iter().map(|month| month.items.len()).sum();
        years.push(YearView {
            year,
            months: month_views,
            item_count,
        });
    }

    Catalog {
        years,
        item_count: total,
        events: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(path: &str, year: i32, month: u32, captured_at: &str) -> CatalogItemInput {
        CatalogItemInput {
            rel_path: path.to_string(),
            year,
            month,
            kind: MediaKind::Image,
            captured_at: Some(captured_at.to_string()),
            mtime_ns: 0,
            width: Some(10),
            height: Some(10),
            duration_ms: None,
            thumb_version: Some("1-2".to_string()),
            thumb_available: true,
        }
    }

    #[test]
    fn years_and_months_are_newest_first() {
        let catalog = build_catalog(
            vec![
                item("2023/12/a.jpg", 2023, 12, "2023-12-01 00:00:00"),
                item("2024/05/a.jpg", 2024, 5, "2024-05-01 00:00:00"),
                item("2024/01/a.jpg", 2024, 1, "2024-01-01 00:00:00"),
            ],
            "/media",
            "/thumbs",
        );
        let years: Vec<_> = catalog.years.iter().map(|year| year.year).collect();
        assert_eq!(years, vec![2024, 2023]);
        let months: Vec<_> = catalog.years[0]
            .months
            .iter()
            .map(|month| month.month)
            .collect();
        assert_eq!(months, vec![5, 1]);
    }

    #[test]
    fn items_are_newest_first_with_path_tie_break() {
        let catalog = build_catalog(
            vec![
                item("2024/05/b.jpg", 2024, 5, "2024-05-01 10:00:00"),
                item("2024/05/a.jpg", 2024, 5, "2024-05-01 10:00:00"),
                item("2024/05/c.jpg", 2024, 5, "2024-05-02 10:00:00"),
            ],
            "/media",
            "/thumbs",
        );
        let paths: Vec<_> = catalog.years[0].months[0]
            .items
            .iter()
            .map(|item| item.path.as_str())
            .collect();
        assert_eq!(
            paths,
            vec!["2024/05/c.jpg", "2024/05/a.jpg", "2024/05/b.jpg"]
        );
    }

    #[test]
    fn missing_timestamp_falls_back_to_mtime() {
        let fallback = capture_timestamp(None, 1_700_000_000_000_000_000);
        assert_eq!(fallback.len(), 19);
        assert!(fallback.starts_with("2023-") || fallback.starts_with("2024-"));
        assert_eq!(
            capture_timestamp(Some("  ".to_string()), 0),
            "1970-01-01 00:00:00"
        );
    }

    #[test]
    fn urls_are_encoded_and_versioned() {
        let catalog = build_catalog(
            vec![item(
                "2024/05/Familie Müller #1.jpg",
                2024,
                5,
                "2024-05-01 00:00:00",
            )],
            "/media",
            "/thumbs",
        );
        let item = &catalog.years[0].months[0].items[0];
        assert_eq!(
            item.media_url,
            "/media/2024/05/Familie%20M%C3%BCller%20%231.jpg"
        );
        assert_eq!(
            item.thumb_url.as_deref(),
            Some("/thumbs/2024/05/Familie%20M%C3%BCller%20%231.jpg.webp?v=1-2")
        );
        assert_eq!(item.name, "Familie Müller #1.jpg");
    }

    #[test]
    fn unavailable_thumb_has_no_url() {
        let catalog = build_catalog(
            vec![CatalogItemInput {
                thumb_available: false,
                ..item("2024/05/a.heic", 2024, 5, "2024-05-01 00:00:00")
            }],
            "/media",
            "/thumbs",
        );
        assert!(catalog.years[0].months[0].items[0].thumb_url.is_none());
    }
}
