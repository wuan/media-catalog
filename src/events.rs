use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};
use chrono::{Duration, NaiveDateTime, Timelike};
use serde::Deserialize;

use crate::model::{Catalog, EventMeta, EventSection, EventView, ItemView, MonthView, SectionView};

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
    #[serde(default)]
    sections: Vec<SectionEntry>,
}

/// An authored bookmark section. `at` is resolved into a published range.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SectionEntry {
    slug: String,
    title: String,
    at: String,
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
        .map(parse_event_entry)
        .collect::<Result<Vec<_>>>()?;
    validate_metadata(&events)?;
    Ok(events)
}

fn parse_event_entry(entry: EventEntry) -> Result<EventMeta> {
    validate_slug(&entry.slug)?;
    if entry.title.trim().is_empty() {
        bail!("event {:?} is missing a title", entry.slug);
    }
    let start = parse_event_timestamp(&entry.start)
        .with_context(|| format!("event {:?} has an invalid start", entry.slug))?;
    let end = parse_event_timestamp(&entry.end)
        .with_context(|| format!("event {:?} has an invalid end", entry.slug))?;
    if start > end {
        bail!(
            "event {:?} starts at {} after it ends at {}",
            entry.slug,
            entry.start,
            entry.end
        );
    }

    Ok(EventMeta {
        slug: entry.slug.clone(),
        title: entry.title,
        start: entry.start,
        end: entry.end,
        description: normalize_description(entry.description),
        item_count: 0,
        sections: resolve_sections(&entry.slug, start, end, entry.sections)?,
    })
}

/// Resolve authored bookmark timestamps into inclusive published ranges.
fn resolve_sections(
    event_slug: &str,
    event_start: NaiveDateTime,
    event_end: NaiveDateTime,
    entries: Vec<SectionEntry>,
) -> Result<Vec<EventSection>> {
    let mut slugs = BTreeSet::new();
    let mut timestamps = BTreeSet::new();
    let mut authored = Vec::with_capacity(entries.len());

    for entry in entries {
        validate_slug(&entry.slug)
            .with_context(|| format!("event {event_slug:?} has an invalid section slug"))?;
        if entry.title.trim().is_empty() {
            bail!(
                "event {event_slug:?} section {:?} is missing a title",
                entry.slug
            );
        }
        let at = parse_event_timestamp(&entry.at).with_context(|| {
            format!(
                "event {event_slug:?} section {:?} has an invalid at timestamp",
                entry.slug
            )
        })?;
        if !slugs.insert(entry.slug.clone()) {
            bail!(
                "event {event_slug:?} has duplicate section slug {:?}",
                entry.slug
            );
        }
        if !timestamps.insert(at) {
            bail!(
                "event {event_slug:?} has duplicate section timestamp {:?}",
                entry.at
            );
        }
        if at < event_start || at > event_end {
            bail!(
                "event {event_slug:?} section {:?} starts at {} outside {} to {}",
                entry.slug,
                entry.at,
                event_start.format(EVENT_TIMESTAMP_FORMAT),
                event_end.format(EVENT_TIMESTAMP_FORMAT)
            );
        }
        authored.push((at, entry));
    }

    authored.sort_by_key(|(at, _)| *at);

    let mut sections = Vec::with_capacity(authored.len());
    for (index, (at, entry)) in authored.iter().enumerate() {
        let end = match authored.get(index + 1) {
            Some((next_at, _)) => *next_at - Duration::minutes(1),
            None => event_end,
        };
        sections.push(EventSection {
            slug: entry.slug.clone(),
            title: entry.title.clone(),
            start: at.format(EVENT_TIMESTAMP_FORMAT).to_string(),
            end: end.format(EVENT_TIMESTAMP_FORMAT).to_string(),
            description: normalize_description(entry.description.clone()),
        });
    }

    Ok(sections)
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

        let mut section_slugs = BTreeSet::new();
        let mut previous_end = None;
        for section in &event.sections {
            validate_slug(&section.slug)
                .with_context(|| format!("event {:?} has an invalid section slug", event.slug))?;
            if !section_slugs.insert(section.slug.as_str()) {
                bail!(
                    "event {:?} has duplicate section slug {:?}",
                    event.slug,
                    section.slug
                );
            }
            if section.title.trim().is_empty() {
                bail!(
                    "event {:?} section {:?} is missing a title",
                    event.slug,
                    section.slug
                );
            }
            let section_start = parse_event_timestamp(&section.start).with_context(|| {
                format!(
                    "event {:?} section {:?} has an invalid start",
                    event.slug, section.slug
                )
            })?;
            let section_end = parse_event_timestamp(&section.end).with_context(|| {
                format!(
                    "event {:?} section {:?} has an invalid end",
                    event.slug, section.slug
                )
            })?;
            if section_start > section_end {
                bail!(
                    "event {:?} section {:?} starts at {} after it ends at {}",
                    event.slug,
                    section.slug,
                    section.start,
                    section.end
                );
            }
            if section_start < start || section_end > end {
                bail!(
                    "event {:?} section {:?} range {} to {} is outside the event range {} to {}",
                    event.slug,
                    section.slug,
                    section.start,
                    section.end,
                    event.start,
                    event.end
                );
            }
            if let Some(previous_end) = previous_end {
                if section_start <= previous_end {
                    bail!(
                        "event {:?} section {:?} overlaps or is out of order at {}",
                        event.slug,
                        section.slug,
                        section.start
                    );
                }
            }
            previous_end = Some(section_end);
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
                let mut items = month
                    .items
                    .iter()
                    .filter(|item| item.events.iter().any(|slug| slug == &event.slug))
                    .cloned()
                    .collect::<Vec<_>>();
                if items.is_empty() {
                    continue;
                }
                items.sort_by(|left, right| {
                    left.captured_at
                        .cmp(&right.captured_at)
                        .then_with(|| left.path.cmp(&right.path))
                });
                months.push(MonthView {
                    year: year.year,
                    month: month.month,
                    label: month.label.clone(),
                    short_label: month.short_label.clone(),
                    items,
                });
            }
        }

        // Month groups follow the same oldest-first direction as the items they contain.
        months.sort_by_key(|month| (month.year, month.month));

        let count = months.iter().map(|month| month.items.len()).sum::<usize>();
        if count != event.item_count {
            bail!(
                "event {:?} declares {} items but the shards contain {}",
                event.slug,
                event.item_count,
                count
            );
        }

        let mut items = months
            .iter()
            .flat_map(|month| month.items.iter())
            .cloned()
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            left.captured_at
                .cmp(&right.captured_at)
                .then_with(|| left.path.cmp(&right.path))
        });
        let sections = event
            .sections
            .iter()
            .map(|section| section_view(section, &items))
            .collect::<Vec<_>>();
        let other_items = items
            .iter()
            .filter(|item| !in_any_section(item, &sections))
            .cloned()
            .collect::<Vec<_>>();

        views.push(EventView {
            slug: event.slug.clone(),
            title: event.title.clone(),
            start: event.start.clone(),
            end: event.end.clone(),
            description: event.description.clone(),
            item_count: count,
            months,
            sections,
            other_items,
        });
    }

    Ok(views)
}

fn section_view(section: &EventSection, items: &[ItemView]) -> SectionView {
    let start = parse_event_timestamp(&section.start).expect("validated section start");
    let end = parse_event_timestamp(&section.end).expect("validated section end");
    let items = items
        .iter()
        .filter(|item| {
            parse_item_timestamp(&item.captured_at)
                .map(|timestamp| start <= timestamp && timestamp <= end)
                .unwrap_or(false)
        })
        .cloned()
        .collect();
    SectionView {
        slug: section.slug.clone(),
        title: section.title.clone(),
        start: section.start.clone(),
        end: section.end.clone(),
        description: section.description.clone(),
        items,
    }
}

fn in_any_section(item: &ItemView, sections: &[SectionView]) -> bool {
    parse_item_timestamp(&item.captured_at)
        .map(|timestamp| {
            sections.iter().any(|section| {
                let start = parse_event_timestamp(&section.start).expect("validated section start");
                let end = parse_event_timestamp(&section.end).expect("validated section end");
                start <= timestamp && timestamp <= end
            })
        })
        .unwrap_or(false)
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
            model: None,
            mtime_ns: 0,
            size: 0,
            width: Some(10),
            height: Some(10),
            duration_ms: None,
            thumb_version: Some("1-2".to_string()),
            thumb_available: true,
            ..CatalogItemInput::default()
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
            sections: Vec::new(),
        }
    }

    fn section(slug: &str, start: &str, end: &str) -> EventSection {
        EventSection {
            slug: slug.to_string(),
            title: format!("Section {slug}"),
            start: start.to_string(),
            end: end.to_string(),
            description: None,
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
    fn parses_and_resolves_bookmark_sections() {
        let text = r#"
            [[events]]
            slug = "portugal"
            title = "Portugal"
            start = "2025-04-12 00:00"
            end = "2025-05-18 23:59"

            [[events.sections]]
            slug = "porto"
            title = "Porto"
            at = "2025-05-10 00:00"

            [[events.sections]]
            slug = "lisbon"
            title = "Lisbon"
            at = "2025-04-12 00:00"

            [[events.sections]]
            slug = "algarve"
            title = "Algarve"
            at = "2025-04-20 00:00"
            description = "  Coast  "
        "#;
        let events = parse_events(text, "events.toml").unwrap();
        assert_eq!(events.len(), 1);
        let sections = &events[0].sections;
        assert_eq!(
            sections
                .iter()
                .map(|section| section.slug.as_str())
                .collect::<Vec<_>>(),
            vec!["lisbon", "algarve", "porto"]
        );
        assert_eq!(sections[0].start, "2025-04-12 00:00");
        assert_eq!(sections[0].end, "2025-04-19 23:59");
        assert_eq!(sections[1].start, "2025-04-20 00:00");
        assert_eq!(sections[1].end, "2025-05-09 23:59");
        assert_eq!(sections[1].description.as_deref(), Some("Coast"));
        assert_eq!(sections[2].start, "2025-05-10 00:00");
        assert_eq!(sections[2].end, "2025-05-18 23:59");
    }

    #[test]
    fn section_authoring_rules_are_validated() {
        let base = r#"
            [[events]]
            slug = "portugal"
            title = "Portugal"
            start = "2025-04-12 00:00"
            end = "2025-05-18 23:59"
        "#;

        let unknown = format!(
            "{base}\n[[events.sections]]\nslug = \"lisbon\"\ntitle = \"Lisbon\"\nat = \"2025-04-12 00:00\"\nlocation = \"Portugal\"\n"
        );
        assert!(
            format!("{:#}", parse_events(&unknown, "events.toml").unwrap_err())
                .contains("unknown field")
        );

        for slug in [
            "Lisbon",
            "lisbon 2025",
            "lisbon_2025",
            "-lisbon",
            "lisbon-",
            "lisbon--2025",
        ] {
            let text = format!(
                "{base}\n[[events.sections]]\nslug = {slug:?}\ntitle = \"Lisbon\"\nat = \"2025-04-12 00:00\"\n"
            );
            assert!(
                parse_events(&text, "events.toml").is_err(),
                "section slug {slug:?} was accepted"
            );
        }

        let blank_title = format!(
            "{base}\n[[events.sections]]\nslug = \"lisbon\"\ntitle = \"   \"\nat = \"2025-04-12 00:00\"\n"
        );
        assert!(parse_events(&blank_title, "events.toml").is_err());

        let malformed = format!(
            "{base}\n[[events.sections]]\nslug = \"lisbon\"\ntitle = \"Lisbon\"\nat = \"2025-04-12T00:00\"\n"
        );
        let error = parse_events(&malformed, "events.toml").unwrap_err();
        assert!(format!("{error:#}").contains("YYYY-MM-DD HH:MM"));

        let outside = format!(
            "{base}\n[[events.sections]]\nslug = \"lisbon\"\ntitle = \"Lisbon\"\nat = \"2025-04-11 23:59\"\n"
        );
        let error = parse_events(&outside, "events.toml").unwrap_err();
        assert!(error.to_string().contains("outside"));

        let duplicate_slug = format!(
            "{base}\n[[events.sections]]\nslug = \"lisbon\"\ntitle = \"Lisbon\"\nat = \"2025-04-12 00:00\"\n\n[[events.sections]]\nslug = \"lisbon\"\ntitle = \"Again\"\nat = \"2025-04-20 00:00\"\n"
        );
        let error = parse_events(&duplicate_slug, "events.toml").unwrap_err();
        assert!(error.to_string().contains("duplicate section slug"));

        let duplicate_timestamp = format!(
            "{base}\n[[events.sections]]\nslug = \"lisbon\"\ntitle = \"Lisbon\"\nat = \"2025-04-12 00:00\"\n\n[[events.sections]]\nslug = \"algarve\"\ntitle = \"Algarve\"\nat = \"2025-04-12 00:00\"\n"
        );
        let error = parse_events(&duplicate_timestamp, "events.toml").unwrap_err();
        assert!(error.to_string().contains("duplicate section timestamp"));
    }

    #[test]
    fn published_sections_are_validated() {
        let mut catalog = catalog(vec![item("2024/05/a.jpg", 2024, 5, "2024-05-10 00:00:00")]);
        let valid = vec![EventMeta {
            sections: vec![
                section("first", "2024-05-01 00:00", "2024-05-09 23:59"),
                section("second", "2024-05-10 00:00", "2024-05-31 23:59"),
            ],
            ..event("trip", "2024-05-01 00:00", "2024-05-31 23:59")
        }];
        attach_events(&mut catalog, valid.clone()).unwrap();
        assert_eq!(catalog.events[0].sections[1].items.len(), 1);

        let cases = [
            (
                vec![
                    section("first", "2024-05-01 00:00", "2024-05-10 00:00"),
                    section("second", "2024-05-10 00:00", "2024-05-31 23:59"),
                ],
                "overlaps or is out of order",
            ),
            (
                vec![
                    section("second", "2024-05-10 00:00", "2024-05-31 23:59"),
                    section("first", "2024-05-01 00:00", "2024-05-09 23:59"),
                ],
                "overlaps or is out of order",
            ),
            (
                vec![section("outside", "2024-04-30 00:00", "2024-05-09 23:59")],
                "outside the event range",
            ),
            (
                vec![section("reversed", "2024-05-10 00:00", "2024-05-09 23:59")],
                "after it ends",
            ),
            (
                vec![
                    section("same", "2024-05-01 00:00", "2024-05-09 23:59"),
                    section("same", "2024-05-10 00:00", "2024-05-31 23:59"),
                ],
                "duplicate section slug",
            ),
        ];

        for (sections, message) in cases {
            let meta = vec![EventMeta {
                sections,
                ..event("trip", "2024-05-01 00:00", "2024-05-31 23:59")
            }];
            let error = validate_metadata(&meta).unwrap_err();
            assert!(
                error.to_string().contains(message),
                "expected {message:?}, got {error}"
            );
        }
    }

    #[test]
    fn section_views_assign_boundaries_and_collect_other_items() {
        let mut sectioned = catalog(vec![
            item("2024/05/before.jpg", 2024, 5, "2024-05-01 00:00:00"),
            item("2024/05/first.jpg", 2024, 5, "2024-05-09 23:59:00"),
            item("2024/05/boundary.jpg", 2024, 5, "2024-05-10 00:00:00"),
            item("2024/05/second.jpg", 2024, 5, "2024-05-15 00:00:00"),
            item("2024/05/gap.jpg", 2024, 5, "2024-05-20 00:00:00"),
        ]);
        attach_events(
            &mut sectioned,
            vec![EventMeta {
                sections: vec![
                    section("first", "2024-05-05 00:00", "2024-05-09 23:59"),
                    section("second", "2024-05-10 00:00", "2024-05-19 23:59"),
                ],
                ..event("trip", "2024-05-01 00:00", "2024-05-31 23:59")
            }],
        )
        .unwrap();

        let view = sectioned.event("trip").unwrap();
        let paths = |items: &[ItemView]| {
            items
                .iter()
                .map(|item| item.path.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            paths(&view.sections[0].items),
            vec!["2024/05/first.jpg".to_string()]
        );
        assert_eq!(
            paths(&view.sections[1].items),
            vec![
                "2024/05/boundary.jpg".to_string(),
                "2024/05/second.jpg".to_string()
            ]
        );
        assert_eq!(
            paths(&view.other_items),
            vec![
                "2024/05/before.jpg".to_string(),
                "2024/05/gap.jpg".to_string()
            ]
        );

        // The partition has no Other bucket when every item is covered.
        let mut covered = catalog(vec![item("2024/05/a.jpg", 2024, 5, "2024-05-10 00:00:00")]);
        attach_events(
            &mut covered,
            vec![EventMeta {
                sections: vec![
                    section("first", "2024-05-01 00:00", "2024-05-09 23:59"),
                    section("second", "2024-05-10 00:00", "2024-05-31 23:59"),
                ],
                ..event("trip", "2024-05-01 00:00", "2024-05-31 23:59")
            }],
        )
        .unwrap();
        assert!(covered.event("trip").unwrap().other_items.is_empty());
    }

    #[test]
    fn readme_event_example_parses_with_sections() {
        let readme = include_str!("../README.md");
        let start = readme.find("[[events]]").expect("README event example");
        let end = start + readme[start..].find("```").expect("README code block end");
        let events = parse_events(&readme[start..end], "README.md").unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0]
                .sections
                .iter()
                .map(|section| section.slug.as_str())
                .collect::<Vec<_>>(),
            vec!["chamonix", "zermatt"]
        );
        assert_eq!(events[0].sections[0].end, "2024-06-01 07:59");
        assert_eq!(events[0].sections[1].end, "2024-06-05 18:30");
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
            model: None,
            mtime_ns: 1_714_610_000_000_000_000, // 2024-05-01 ~ in local time
            size: 0,
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
    fn event_month_groups_are_ordered_oldest_first() {
        let mut catalog = catalog(vec![
            item("2024/06/b.jpg", 2024, 6, "2024-06-02 10:00:00"),
            item("2024/05/a.jpg", 2024, 5, "2024-05-03 10:00:00"),
        ]);
        attach_events(
            &mut catalog,
            vec![event("trip", "2024-05-01 00:00", "2024-06-30 23:59")],
        )
        .unwrap();

        let months = catalog
            .event("trip")
            .unwrap()
            .months
            .iter()
            .map(|month| format!("{}-{:02}", month.year, month.month))
            .collect::<Vec<_>>();
        assert_eq!(months, vec!["2024-05", "2024-06"]);
    }

    #[test]
    fn event_month_groups_sort_by_year_then_month() {
        let mut catalog = catalog(vec![
            item("2024/02/c.jpg", 2024, 2, "2024-02-05 10:00:00"),
            item("2024/01/b.jpg", 2024, 1, "2024-01-10 10:00:00"),
            item("2023/12/a.jpg", 2023, 12, "2023-12-20 10:00:00"),
        ]);
        attach_events(
            &mut catalog,
            vec![event("winter", "2023-12-01 00:00", "2024-02-28 23:59")],
        )
        .unwrap();

        let months = catalog
            .event("winter")
            .unwrap()
            .months
            .iter()
            .map(|month| format!("{}-{:02}", month.year, month.month))
            .collect::<Vec<_>>();
        assert_eq!(months, vec!["2023-12", "2024-01", "2024-02"]);
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
        assert_eq!(months, vec!["2024-05", "2024-06"]);
        let june = newer.months[1]
            .items
            .iter()
            .map(|item| item.path.as_str())
            .collect::<Vec<_>>();
        assert_eq!(june, vec!["2024/06/d.jpg", "2024/06/c.jpg"]);

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
            sections: Vec::new(),
            other_items: Vec::new(),
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
                sections: view
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
