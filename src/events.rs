use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};
use chrono::{NaiveDateTime, Timelike};
use serde::Deserialize;

use crate::model::{Catalog, EventMeta, EventView, MonthView};

const EVENT_TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M";
const ITEM_TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

/// Curated events above this many items are reported as too large for one page.
pub const LARGE_EVENT_ITEM_LIMIT: usize = 500;

/// Warning text for an event that exceeds the curated single-page size.
pub fn large_event_warning(event: &EventView) -> Option<String> {
    (event.item_count > LARGE_EVENT_ITEM_LIMIT).then(|| {
        format!(
            "event {:?} matched {} items (more than {LARGE_EVENT_ITEM_LIMIT}); consider splitting the event or adding per-month event pages",
            event.slug, event.item_count
        )
    })
}

/// Optional `events.toml` document. A missing file means a catalog without events.
#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct EventsDocument {
    events: Vec<EventEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EventEntry {
    slug: String,
    title: String,
    start: String,
    end: String,
    #[serde(default)]
    description: Option<String>,
}

/// Load authored events, or return an empty list when no file is configured.
pub fn load_events(path: Option<&Path>) -> Result<Vec<EventMeta>> {
    let Some(path) = path else {
        return Ok(Vec::new());
    };
    let text = fs::read_to_string(path)
        .with_context(|| format!("reading events file {}", path.display()))?;
    parse_events(&text, &path.display().to_string())
}

/// Parse and validate an `events.toml` document.
pub fn parse_events(text: &str, source: &str) -> Result<Vec<EventMeta>> {
    let document: EventsDocument =
        toml::from_str(text).with_context(|| format!("parsing events file {source}"))?;
    let events = document
        .events
        .into_iter()
        .map(|entry| EventMeta {
            slug: entry.slug,
            title: entry.title,
            start: entry.start,
            end: entry.end,
            description: normalize_description(entry.description),
            item_count: 0,
        })
        .collect::<Vec<_>>();
    validate_metadata(&events)?;
    Ok(events)
}

/// Validate event definitions that the generator or a published index supplies.
pub fn validate_metadata(events: &[EventMeta]) -> Result<()> {
    let mut seen = BTreeSet::new();
    for event in events {
        validate_slug(&event.slug)?;
        if !seen.insert(event.slug.as_str()) {
            bail!("duplicate event slug {:?}", event.slug);
        }
        if event.title.trim().is_empty() {
            bail!("event {:?} is missing a title", event.slug);
        }
        let start = parse_event_timestamp(&event.start)
            .with_context(|| format!("event {:?} has an invalid start", event.slug))?;
        let end = parse_event_timestamp(&event.end)
            .with_context(|| format!("event {:?} has an invalid end", event.slug))?;
        if start > end {
            bail!(
                "event {:?} starts at {} after it ends at {}",
                event.slug,
                event.start,
                event.end
            );
        }
    }
    Ok(())
}

/// Slugs are stable URL segments: lowercase ASCII letters, digits, and single hyphens.
pub fn validate_slug(slug: &str) -> Result<()> {
    let valid = !slug.is_empty()
        && !slug.starts_with('-')
        && !slug.ends_with('-')
        && !slug.contains("--")
        && slug
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !valid {
        bail!("event slug {slug:?} must use lowercase ASCII letters, digits, and single hyphens");
    }
    Ok(())
}

/// Assign membership by effective timestamp, then rebuild all event views.
///
/// The year/month hierarchy is untouched: only each item's `events` list is set.
pub fn attach_events(catalog: &mut Catalog, mut events: Vec<EventMeta>) -> Result<()> {
    validate_metadata(&events)?;
    events.sort_by(|left, right| {
        right
            .start
            .cmp(&left.start)
            .then_with(|| left.slug.cmp(&right.slug))
    });

    let ranges = events
        .iter()
        .map(|event| {
            (
                parse_event_timestamp(&event.start).expect("validated start timestamp"),
                parse_event_timestamp(&event.end).expect("validated end timestamp"),
            )
        })
        .collect::<Vec<_>>();

    let mut counts = vec![0usize; events.len()];
    for year in &mut catalog.years {
        for month in &mut year.months {
            for item in &mut month.items {
                let timestamp = parse_item_timestamp(&item.captured_at);
                let mut slugs = Vec::new();
                if let Some(timestamp) = timestamp {
                    for (index, (start, end)) in ranges.iter().enumerate() {
                        if *start <= timestamp && timestamp <= *end {
                            slugs.push(events[index].slug.clone());
                            counts[index] += 1;
                        }
                    }
                }
                slugs.sort();
                item.events = slugs;
            }
        }
    }

    for (event, count) in events.iter_mut().zip(counts) {
        event.item_count = count;
    }

    catalog.events = build_event_views(catalog, &events)?;
    Ok(())
}

/// Rebuild event views from item membership and validate the published data.
///
/// This is also the snapshot-load check: undeclared slugs, duplicate definitions,
/// and declared counts that disagree with shard membership are rejected.
pub fn build_event_views(catalog: &Catalog, events: &[EventMeta]) -> Result<Vec<EventView>> {
    validate_metadata(events)?;

    let declared = events
        .iter()
        .map(|event| event.slug.as_str())
        .collect::<BTreeSet<_>>();
    for year in &catalog.years {
        for month in &year.months {
            for item in &month.items {
                for slug in &item.events {
                    if !declared.contains(slug.as_str()) {
                        bail!(
                            "item {:?} references undeclared event slug {:?}",
                            item.path,
                            slug
                        );
                    }
                }
            }
        }
    }

    let mut sorted = events.iter().collect::<Vec<_>>();
    sorted.sort_by(|left, right| {
        right
            .start
            .cmp(&left.start)
            .then_with(|| left.slug.cmp(&right.slug))
    });

    let mut views = Vec::with_capacity(sorted.len());
    for event in sorted {
        let mut months = Vec::new();
        for year in &catalog.years {
            for month in &year.months {
                let items = month
                    .items
                    .iter()
                    .filter(|item| item.events.iter().any(|slug| slug == &event.slug))
                    .cloned()
                    .collect::<Vec<_>>();
                if items.is_empty() {
                    continue;
                }
                months.push(MonthView {
                    year: year.year,
                    month: month.month,
                    label: month.label.clone(),
                    short_label: month.short_label.clone(),
                    items,
                });
            }
        }

        let count = months.iter().map(|month| month.items.len()).sum::<usize>();
        if count != event.item_count {
            bail!(
                "event {:?} declares {} items but the shards contain {}",
                event.slug,
                event.item_count,
                count
            );
        }

        views.push(EventView {
            slug: event.slug.clone(),
            title: event.title.clone(),
            start: event.start.clone(),
            end: event.end.clone(),
            description: event.description.clone(),
            item_count: count,
            months,
        });
    }

    Ok(views)
}

fn parse_event_timestamp(value: &str) -> Result<NaiveDateTime> {
    let parsed = NaiveDateTime::parse_from_str(value, EVENT_TIMESTAMP_FORMAT)
        .with_context(|| format!("expected a timestamp in YYYY-MM-DD HH:MM form, got {value:?}"))?;
    let canonical = parsed.format(EVENT_TIMESTAMP_FORMAT).to_string();
    if canonical != value {
        bail!("expected a timestamp in YYYY-MM-DD HH:MM form, got {value:?}");
    }
    Ok(parsed)
}

fn parse_item_timestamp(value: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(value.trim(), ITEM_TIMESTAMP_FORMAT)
        .ok()
        .map(truncate_to_minute)
}

fn truncate_to_minute(timestamp: NaiveDateTime) -> NaiveDateTime {
    timestamp
        .with_second(0)
        .and_then(|timestamp| timestamp.with_nanosecond(0))
        .unwrap_or(timestamp)
}

fn normalize_description(description: Option<String>) -> Option<String> {
    description
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{build_catalog, CatalogItemInput};
    use crate::scan::MediaKind;

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

    fn event(slug: &str, start: &str, end: &str) -> EventMeta {
        EventMeta {
            slug: slug.to_string(),
            title: format!("Event {slug}"),
            start: start.to_string(),
            end: end.to_string(),
            description: None,
            item_count: 0,
        }
    }

    fn catalog(items: Vec<CatalogItemInput>) -> Catalog {
        build_catalog(items, "/media", "/thumbs")
    }

    #[test]
    fn parses_a_valid_file_and_unknown_fields_are_rejected() {
        let text = r#"
            [[events]]
            slug = "alps-2024"
            title = "Alps trip"
            start = "2024-05-28 09:00"
            end = "2024-06-05 18:30"
            description = "Hiking."

            [[events]]
            slug = "alps-2025"
            title = "Alps trip again"
            start = "2025-05-28 09:00"
            end = "2025-06-05 18:30"
        "#;
        let events = parse_events(text, "events.toml").unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].description.as_deref(), Some("Hiking."));

        let unknown = r#"
            [[events]]
            slug = "alps-2024"
            title = "Alps trip"
            start = "2024-05-28 09:00"
            end = "2024-06-05 18:30"
            location = "Chamonix"
        "#;
        let error = parse_events(unknown, "events.toml").unwrap_err();
        assert!(format!("{error:#}").contains("unknown field"));
    }

    #[test]
    fn missing_explicit_file_is_reported() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("missing.toml");
        let error = load_events(Some(&path)).unwrap_err();
        assert!(format!("{error:#}").contains("missing.toml"));
    }

    #[test]
    fn validates_slugs_titles_timestamps_and_ranges() {
        let duplicate = r#"
            [[events]]
            slug = "trip"
            title = "One"
            start = "2024-05-01 09:00"
            end = "2024-05-02 09:00"

            [[events]]
            slug = "trip"
            title = "Two"
            start = "2024-06-01 09:00"
            end = "2024-06-02 09:00"
        "#;
        let error = parse_events(duplicate, "events.toml").unwrap_err();
        assert!(error.to_string().contains("duplicate event slug"));

        for slug in [
            "Trip",
            "trip 2024",
            "trip_2024",
            "-trip",
            "trip-",
            "trip--2024",
        ] {
            let text = format!(
                "[[events]]\nslug = {slug:?}\ntitle = \"Trip\"\nstart = \"2024-05-01 09:00\"\nend = \"2024-05-02 09:00\"\n"
            );
            assert!(
                parse_events(&text, "events.toml").is_err(),
                "slug {slug:?} was accepted"
            );
        }

        let blank_title = r#"
            [[events]]
            slug = "trip"
            title = "   "
            start = "2024-05-01 09:00"
            end = "2024-05-02 09:00"
        "#;
        assert!(parse_events(blank_title, "events.toml").is_err());

        let reversed = r#"
            [[events]]
            slug = "trip"
            title = "Trip"
            start = "2024-05-02 09:00"
            end = "2024-05-01 09:00"
        "#;
        let error = parse_events(reversed, "events.toml").unwrap_err();
        assert!(error.to_string().contains("after it ends"));

        let malformed = r#"
            [[events]]
            slug = "trip"
            title = "Trip"
            start = "2024-05-01T09:00"
            end = "2024-05-02 09:00"
        "#;
        let error = parse_events(malformed, "events.toml").unwrap_err();
        assert!(format!("{error:#}").contains("YYYY-MM-DD HH:MM"));
    }

    #[test]
    fn membership_is_inclusive_and_truncated_to_the_minute() {
        let mut catalog = catalog(vec![
            item("2024/05/start.jpg", 2024, 5, "2024-05-28 09:00:00"),
            item("2024/05/before.jpg", 2024, 5, "2024-05-28 08:59:59"),
            item("2024/06/end.jpg", 2024, 6, "2024-06-05 18:30:45"),
            item("2024/06/after.jpg", 2024, 6, "2024-06-05 18:31:00"),
        ]);
        attach_events(
            &mut catalog,
            vec![event("trip", "2024-05-28 09:00", "2024-06-05 18:30")],
        )
        .unwrap();

        assert_eq!(catalog.events[0].item_count, 2);
        assert_eq!(
            catalog.months_items("2024/05/start.jpg"),
            vec!["trip".to_string()]
        );
        assert!(catalog.months_items("2024/05/before.jpg").is_empty());
        assert_eq!(
            catalog.months_items("2024/06/end.jpg"),
            vec!["trip".to_string()]
        );
        assert!(catalog.months_items("2024/06/after.jpg").is_empty());
    }

    #[test]
    fn membership_follows_the_timestamp_not_the_folder_and_allows_overlap() {
        let mut catalog = catalog(vec![item(
            "2024/06/may-photo.jpg",
            2024,
            6,
            "2024-05-20 12:00:00",
        )]);
        attach_events(
            &mut catalog,
            vec![
                event("spring", "2024-05-01 00:00", "2024-05-31 23:59"),
                event("may-trip", "2024-05-15 00:00", "2024-05-25 23:59"),
            ],
        )
        .unwrap();

        let slugs = catalog.months_items("2024/06/may-photo.jpg");
        assert_eq!(slugs, vec!["may-trip".to_string(), "spring".to_string()]);
        // Folder grouping stays authoritative for year/month browsing.
        assert!(catalog.month(2024, 6).is_some());
        assert!(catalog.month(2024, 5).is_none());
        assert_eq!(catalog.events[0].slug, "may-trip");
        assert_eq!(catalog.events[1].slug, "spring");
    }

    #[test]
    fn missing_timestamp_falls_back_to_mtime() {
        let mut catalog = catalog(vec![CatalogItemInput {
            captured_at: None,
            mtime_ns: 1_714_610_000_000_000_000, // 2024-05-01 ~ in local time
            ..item("2024/05/fallback.jpg", 2024, 5, "2024-05-01 00:00:00")
        }]);
        let captured = catalog.month(2024, 5).unwrap().items[0].captured_at.clone();
        let day = &captured[..10];
        let mut event = event("day", &format!("{day} 00:00"), &format!("{day} 23:59"));
        event.title = "Day".to_string();
        attach_events(&mut catalog, vec![event]).unwrap();
        assert_eq!(catalog.events[0].item_count, 1);
    }

    #[test]
    fn events_and_items_are_ordered_deterministically() {
        let mut catalog = catalog(vec![
            item("2024/05/a.jpg", 2024, 5, "2024-05-03 10:00:00"),
            item("2024/05/b.jpg", 2024, 5, "2024-05-02 10:00:00"),
            item("2024/06/c.jpg", 2024, 6, "2024-06-02 10:00:00"),
            item("2024/06/d.jpg", 2024, 6, "2024-06-01 10:00:00"),
        ]);
        attach_events(
            &mut catalog,
            vec![
                event("older", "2024-05-01 00:00", "2024-06-30 23:59"),
                event("newer-b", "2024-05-01 00:00", "2024-06-30 23:59"),
                event("newer-a", "2024-05-01 00:00", "2024-06-30 23:59"),
            ],
        )
        .unwrap();

        let slugs = catalog
            .events
            .iter()
            .map(|event| event.slug.as_str())
            .collect::<Vec<_>>();
        assert_eq!(slugs, vec!["newer-a", "newer-b", "older"]);

        let newer = catalog.event("newer-a").unwrap();
        let months = newer
            .months
            .iter()
            .map(|month| format!("{}-{:02}", month.year, month.month))
            .collect::<Vec<_>>();
        assert_eq!(months, vec!["2024-06", "2024-05"]);
        let june = newer.months[0]
            .items
            .iter()
            .map(|item| item.path.as_str())
            .collect::<Vec<_>>();
        assert_eq!(june, vec!["2024/06/c.jpg", "2024/06/d.jpg"]);

        // Config order never leaks into item slug lists.
        let older = catalog.event("older").unwrap();
        assert_eq!(
            older.months[0].items[0].events,
            vec![
                "newer-a".to_string(),
                "newer-b".to_string(),
                "older".to_string()
            ]
        );
    }

    #[test]
    fn warns_only_when_an_event_exceeds_the_limit() {
        let view = EventView {
            slug: "big".into(),
            title: "Big".into(),
            start: "2024-05-01 00:00".into(),
            end: "2024-05-31 23:59".into(),
            description: None,
            item_count: LARGE_EVENT_ITEM_LIMIT,
            months: Vec::new(),
        };
        assert!(large_event_warning(&view).is_none());

        let view = EventView {
            item_count: LARGE_EVENT_ITEM_LIMIT + 1,
            ..view
        };
        let warning = large_event_warning(&view).expect("large event warning");
        assert!(warning.contains("\"big\""));
        assert!(warning.contains("501"));
        assert!(warning.contains("500"));
    }

    #[test]
    fn rejects_undeclared_membership_and_count_mismatches() {
        let mut catalog = catalog(vec![item("2024/05/a.jpg", 2024, 5, "2024-05-01 10:00:00")]);
        attach_events(
            &mut catalog,
            vec![event("trip", "2024-05-01 00:00", "2024-05-31 23:59")],
        )
        .unwrap();

        let mut broken = catalog.clone();
        broken.years[0].months[0].items[0].events = vec!["ghost".to_string()];
        let undecided = catalog.events.clone();
        let error = build_event_views(&broken, &published(&undecided)).unwrap_err();
        assert!(error.to_string().contains("undeclared event slug"));

        let mut definitions = published(&catalog.events);
        definitions[0].item_count += 1;
        let error = build_event_views(&catalog, &definitions).unwrap_err();
        assert!(error.to_string().contains("declares 2 items"));
    }

    fn published(views: &[EventView]) -> Vec<EventMeta> {
        views
            .iter()
            .map(|view| EventMeta {
                slug: view.slug.clone(),
                title: view.title.clone(),
                start: view.start.clone(),
                end: view.end.clone(),
                description: view.description.clone(),
                item_count: view.item_count,
            })
            .collect()
    }

    impl Catalog {
        fn months_items(&self, path: &str) -> Vec<String> {
            self.years
                .iter()
                .flat_map(|year| year.months.iter())
                .flat_map(|month| month.items.iter())
                .find(|item| item.path == path)
                .map(|item| item.events.clone())
                .unwrap_or_default()
        }
    }
}
