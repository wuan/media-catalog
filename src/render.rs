use anyhow::{Context, Result};
use askama::Template;
use chrono::NaiveDateTime;

use crate::model::{Catalog, EventView, ItemView};
use crate::paths::{event_page_url, month_page_url, year_page_url};
use crate::scan::MediaKind;

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    years: Vec<YearLink>,
    events: Vec<EventLink>,
    total_items: usize,
}

struct YearLink {
    year: i32,
    count: usize,
    url: String,
}

#[derive(Template)]
#[template(path = "events.html")]
struct EventsTemplate {
    events: Vec<EventLink>,
}

struct EventLink {
    title: String,
    range: String,
    description: String,
    count: usize,
    url: String,
}

#[derive(Template)]
#[template(path = "event.html")]
struct EventTemplate {
    title: String,
    range: String,
    description: String,
    item_count: usize,
    months: Vec<EventMonthSection>,
    has_sections: bool,
    section_nav: Vec<SectionLink>,
    sections: Vec<EventSectionSection>,
    other_items: Vec<TileView>,
}

struct EventMonthSection {
    label: String,
    url: String,
    count: usize,
    items: Vec<TileView>,
}

struct SectionLink {
    slug: String,
    title: String,
    count: usize,
}

struct EventSectionSection {
    slug: String,
    title: String,
    range: String,
    description: String,
    count: usize,
    items: Vec<TileView>,
}

#[derive(Template)]
#[template(path = "year.html")]
struct YearTemplate {
    year: i32,
    total_items: usize,
    months: Vec<MonthLink>,
}

struct MonthLink {
    month: String,
    count: usize,
    url: String,
}

#[derive(Template)]
#[template(path = "month.html")]
struct MonthTemplate {
    year: i32,
    month_label: String,
    year_url: String,
    item_count: usize,
    items: Vec<TileView>,
}

struct TileView {
    media_url: String,
    rel_path: String,
    thumb_url: String,
    thumb_available: bool,
    alt: String,
    kind: &'static str,
    is_video: bool,
    duration_label: String,
    captured_label: String,
    caption_time: String,
    model: String,
    size: Option<u64>,
    width: Option<u32>,
    height: Option<u32>,
}

#[derive(Template)]
#[template(path = "search.html")]
struct SearchTemplate;

/// Render the root years index, including events when present.
pub fn render_index(catalog: &Catalog) -> Result<String> {
    IndexTemplate {
        years: catalog
            .years
            .iter()
            .map(|year| YearLink {
                year: year.year,
                count: year.item_count,
                url: year_page_url(year.year),
            })
            .collect(),
        events: catalog.events.iter().map(event_link).collect(),
        total_items: catalog.item_count,
    }
    .render()
    .context("rendering root index")
}

/// Render the canonical event list at `/events/`.
pub fn render_events(catalog: &Catalog) -> Result<String> {
    EventsTemplate {
        events: catalog.events.iter().map(event_link).collect(),
    }
    .render()
    .context("rendering event list")
}

/// Render one event page, or `None` when the slug is not declared.
pub fn render_event(catalog: &Catalog, slug: &str) -> Result<Option<String>> {
    let Some(event) = catalog.event(slug) else {
        return Ok(None);
    };
    let sections = event
        .sections
        .iter()
        .filter(|section| !section.items.is_empty())
        .map(|section| EventSectionSection {
            slug: section.slug.clone(),
            title: section.title.clone(),
            range: format!("{} to {}", section.start, section.end),
            description: section.description.clone().unwrap_or_default(),
            count: section.items.len(),
            items: section.items.iter().map(tile_from_item).collect(),
        })
        .collect::<Vec<_>>();
    let mut section_nav: Vec<SectionLink> = Vec::new();
    if !event.other_items.is_empty() {
        section_nav.push(SectionLink {
            slug: "other".into(),
            title: "Other".into(),
            count: event.other_items.len(),
        });
    }
    section_nav.extend(sections.iter().map(|section| SectionLink {
        slug: section.slug.clone(),
        title: section.title.clone(),
        count: section.count,
    }));
    let html = EventTemplate {
        title: event.title.clone(),
        range: event_range(event),
        description: event.description.clone().unwrap_or_default(),
        item_count: event.item_count,
        months: event
            .months
            .iter()
            .map(|month| EventMonthSection {
                label: month.label.clone(),
                url: month_page_url(month.year, month.month),
                count: month.items.len(),
                items: month.items.iter().map(tile_from_item).collect(),
            })
            .collect(),
        has_sections: !event.sections.is_empty(),
        section_nav,
        sections,
        other_items: event.other_items.iter().map(tile_from_item).collect(),
    }
    .render()
    .with_context(|| format!("rendering event {}", event.slug))?;
    Ok(Some(html))
}

fn event_link(event: &EventView) -> EventLink {
    EventLink {
        title: event.title.clone(),
        range: event_range(event),
        description: event.description.clone().unwrap_or_default(),
        count: event.item_count,
        url: event_page_url(&event.slug),
    }
}

fn event_range(event: &EventView) -> String {
    format!("{} to {}", event.start, event.end)
}

/// Render the months index for a year, or `None` when the catalog has no such year.
pub fn render_year(catalog: &Catalog, year: i32) -> Result<Option<String>> {
    let Some(view) = catalog.year(year) else {
        return Ok(None);
    };
    let html = YearTemplate {
        year: view.year,
        total_items: view.item_count,
        months: view
            .months
            .iter()
            .map(|month| MonthLink {
                month: month.short_label.clone(),
                count: month.items.len(),
                url: month_page_url(view.year, month.month),
            })
            .collect(),
    }
    .render()
    .with_context(|| format!("rendering year {}", view.year))?;
    Ok(Some(html))
}

/// Render the item grid for a year and month, or `None` when it is absent.
pub fn render_month(catalog: &Catalog, year: i32, month: u32) -> Result<Option<String>> {
    let Some(view) = catalog.month(year, month) else {
        return Ok(None);
    };
    let html = MonthTemplate {
        year: view.year,
        month_label: view.label.clone(),
        year_url: year_page_url(view.year),
        item_count: view.items.len(),
        items: view.items.iter().map(tile_from_item).collect(),
    }
    .render()
    .with_context(|| format!("rendering {}-{:02}", view.year, view.month))?;
    Ok(Some(html))
}

/// Render the embedded search and filter shell.
pub fn render_search_shell() -> Result<String> {
    SearchTemplate.render().context("rendering search page")
}

fn tile_from_item(item: &ItemView) -> TileView {
    let alt = if item.name.trim().is_empty() {
        format!("Media item captured {}", item.captured_at)
    } else {
        item.name.clone()
    };
    TileView {
        media_url: item.media_url.clone(),
        rel_path: item.path.clone(),
        thumb_url: item.thumb_url.clone().unwrap_or_default(),
        thumb_available: item.thumb_available && item.thumb_url.is_some(),
        alt,
        kind: item.kind.as_str(),
        is_video: item.kind == MediaKind::Video,
        duration_label: item.duration_ms.map(format_duration).unwrap_or_default(),
        captured_label: item.captured_at.clone(),
        caption_time: short_capture_label(&item.captured_at),
        model: item.model.clone().unwrap_or_default(),
        size: item.size,
        width: item.width,
        height: item.height,
    }
}

/// Format the canonical capture timestamp for a tile caption without changing
/// the stored value used for ordering, event matching, or viewer metadata.
fn short_capture_label(raw: &str) -> String {
    NaiveDateTime::parse_from_str(raw.trim(), "%Y-%m-%d %H:%M:%S")
        .map(|timestamp| timestamp.format("%y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|_| raw.to_string())
}

pub fn format_duration(duration_ms: u64) -> String {
    let total_seconds = duration_ms / 1000;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{build_catalog, CatalogItemInput, EventMeta, EventSection};

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
            width: Some(4000),
            height: Some(3000),
            duration_ms: None,
            thumb_version: Some("10-20".to_string()),
            thumb_available: true,
        }
    }

    #[test]
    fn snapshot_empty_catalog() {
        let catalog = build_catalog(vec![], "/media", "/thumbs");
        insta::assert_snapshot!("index_empty", render_index(&catalog).unwrap());
    }

    #[test]
    fn snapshot_single_year() {
        let catalog = build_catalog(
            vec![item("2024/05/a.jpg", 2024, 5, "2024-05-01 10:00:00")],
            "/media",
            "/thumbs",
        );
        insta::assert_snapshot!("index_single_year", render_index(&catalog).unwrap());
        insta::assert_snapshot!("year_2024", render_year(&catalog, 2024).unwrap().unwrap());
        insta::assert_snapshot!(
            "month_2024_05",
            render_month(&catalog, 2024, 5).unwrap().unwrap()
        );
    }

    #[test]
    fn snapshot_multi_year() {
        let catalog = build_catalog(
            vec![
                item("2023/12/a.jpg", 2023, 12, "2023-12-01 10:00:00"),
                item("2024/05/b.jpg", 2024, 5, "2024-05-01 10:00:00"),
            ],
            "/media",
            "/thumbs",
        );
        insta::assert_snapshot!("index_multi_year", render_index(&catalog).unwrap());
    }

    #[test]
    fn unknown_year_and_month_have_no_page() {
        let catalog = build_catalog(
            vec![item("2024/05/a.jpg", 2024, 5, "2024-05-01 10:00:00")],
            "/media",
            "/thumbs",
        );
        assert!(render_year(&catalog, 2023).unwrap().is_none());
        assert!(render_month(&catalog, 2024, 6).unwrap().is_none());
    }

    #[test]
    fn month_grid_is_bounded_and_links_every_item() {
        let items: Vec<_> = (0..500)
            .map(|index| {
                item(
                    &format!("2024/05/item-{index:03}.jpg"),
                    2024,
                    5,
                    "2024-05-01 10:00:00",
                )
            })
            .collect();
        let catalog = build_catalog(items, "/media", "/thumbs");
        let html = render_month(&catalog, 2024, 5).unwrap().unwrap();
        assert!(
            html.len() < 2_000_000,
            "month page was {} bytes",
            html.len()
        );
        assert_eq!(html.matches("href=\"/media/").count(), 500);
        assert_eq!(html.matches("loading=\"lazy\"").count(), 500);
    }

    #[test]
    fn tile_caption_shows_short_time_and_model() {
        let catalog = build_catalog(
            vec![
                CatalogItemInput {
                    model: Some("iPhone 15 Pro".into()),
                    ..item("2024/05/a.jpg", 2024, 5, "2024-05-01 10:00:00")
                },
                item("2024/05/b.jpg", 2024, 5, "2024-05-02 10:00:00"),
            ],
            "/media",
            "/thumbs",
        );
        let html = render_month(&catalog, 2024, 5).unwrap().unwrap();
        assert!(html.contains("<time>24-05-01 10:00</time>"));
        assert!(html.contains("<span class=\"model\">iPhone 15 Pro</span>"));
        assert!(html.contains("<time>24-05-02 10:00</time>"));
        assert_eq!(html.matches("class=\"model\"").count(), 1);
        assert!(html.contains("data-captured=\"2024-05-01 10:00:00\""));
        assert!(html.contains("alt=\"a.jpg\""));
        assert!(!html.contains("<span class=\"name\">"));
    }

    #[test]
    fn tiles_carry_the_file_size_only_when_known() {
        let mut catalog = build_catalog(
            vec![
                CatalogItemInput {
                    size: 3_400_000,
                    ..item("2024/05/a.jpg", 2024, 5, "2024-05-01 10:00:00")
                },
                item("2024/05/b.jpg", 2024, 5, "2024-05-02 10:00:00"),
            ],
            "/media",
            "/thumbs",
        );
        // Items loaded from shards published before sizes were recorded have
        // no size; build_catalog always knows it, so clear it by hand.
        for year in &mut catalog.years {
            for month in &mut year.months {
                for item in &mut month.items {
                    if item.path == "2024/05/b.jpg" {
                        item.size = None;
                    }
                }
            }
        }
        let html = render_month(&catalog, 2024, 5).unwrap().unwrap();
        assert!(html.contains("data-size=\"3400000\""));
        assert_eq!(html.matches("data-size=").count(), 1);
        assert!(!html.contains("3.4 MB"), "tile captions do not show the size");
    }

    #[test]
    fn escapes_markup_and_alt_text_is_non_empty() {
        let catalog = build_catalog(
            vec![CatalogItemInput {
                model: Some("<script>alert(2)</script>".into()),
                ..item(
                    "2024/05/<script>alert(1).jpg",
                    2024,
                    5,
                    "2024-05-01 10:00:00",
                )
            }],
            "/media",
            "/thumbs",
        );
        let html = render_month(&catalog, 2024, 5).unwrap().unwrap();
        assert!(!html.contains("<script>alert(1)"));
        assert!(!html.contains("<script>alert(2)"));
        assert!(
            html.contains("&#60;script&#62;alert(1).jpg")
                || html.contains("&lt;script&gt;alert(1).jpg")
        );
        assert!(
            html.contains("alt=\"&#60;script&#62;alert(1).jpg\"")
                || html.contains("alt=\"&lt;script&gt;alert(1).jpg\"")
        );
        assert!(
            html.contains("&#60;script&#62;alert(2)&#60;/script&#62;")
                || html.contains("&lt;script&gt;alert(2)&lt;/script&gt;")
        );
    }

    #[test]
    fn missing_thumb_renders_placeholder_with_label() {
        let catalog = build_catalog(
            vec![CatalogItemInput {
                thumb_available: false,
                thumb_version: None,
                ..item("2024/05/a.heic", 2024, 5, "2024-05-01 10:00:00")
            }],
            "/media",
            "/thumbs",
        );
        let html = render_month(&catalog, 2024, 5).unwrap().unwrap();
        assert!(html.contains("class=\"placeholder\""));
        assert!(html.contains("aria-label=\"a.heic\""));
        assert!(!html.contains("<img"));
    }

    #[test]
    fn video_marker_and_duration_are_rendered() {
        let catalog = build_catalog(
            vec![CatalogItemInput {
                kind: MediaKind::Video,
                duration_ms: Some(65_000),
                ..item("2024/05/clip.mp4", 2024, 5, "2024-05-01 10:00:00")
            }],
            "/media",
            "/thumbs",
        );
        let html = render_month(&catalog, 2024, 5).unwrap().unwrap();
        assert!(html.contains("Video"));
        assert!(html.contains("1:05"));
    }

    #[test]
    fn search_shell_links_back_to_the_hierarchy() {
        let html = render_search_shell().unwrap();
        assert!(html.contains("<noscript>"));
        assert!(html.contains("href=\"/\""));
        assert!(html.contains("assets/search.js"));
    }

    #[test]
    fn tiles_expose_viewer_metadata() {
        let catalog = event_catalog(
            vec![
                CatalogItemInput {
                    kind: MediaKind::Video,
                    duration_ms: Some(65_000),
                    ..item("2024/05/clip.mp4", 2024, 5, "2024-05-01 10:00:00")
                },
                item("2024/05/a.jpg", 2024, 5, "2024-05-02 10:00:00"),
            ],
            vec![event_meta(
                "trip",
                "Trip",
                "2024-05-01 00:00",
                "2024-05-31 23:59",
            )],
        );

        let month = render_month(&catalog, 2024, 5).unwrap().unwrap();
        assert!(month.contains("data-item-path=\"2024/05/a.jpg\""));
        assert!(month.contains("data-kind=\"image\""));
        assert!(month.contains("data-captured=\"2024-05-02 10:00:00\""));
        assert!(month.contains("data-thumb-url=\"/thumbs/2024/05/a.jpg.webp?v=10-20\""));
        assert!(month.contains("data-width=\"4000\""));
        assert!(month.contains("data-height=\"3000\""));
        assert!(month.contains("data-item-path=\"2024/05/clip.mp4\""));
        assert!(month.contains("data-kind=\"video\""));

        let event = render_event(&catalog, "trip").unwrap().unwrap();
        assert!(event.contains("data-item-path=\"2024/05/clip.mp4\""));
        assert!(event.contains("data-kind=\"video\""));
        assert!(event.contains("data-width=\"4000\""));
    }

    #[test]
    fn pages_load_the_viewer_script_once() {
        let catalog = cross_month_catalog();
        let month = render_month(&catalog, 2024, 5).unwrap().unwrap();
        let event = render_event(&catalog, "may-trip").unwrap().unwrap();
        let search = render_search_shell().unwrap();
        for (label, html) in [("month", month), ("event", event), ("search", search)] {
            assert_eq!(
                html.matches("/assets/viewer.js").count(),
                1,
                "{label} page must load the viewer once"
            );
        }
    }

    fn event_meta(slug: &str, title: &str, start: &str, end: &str) -> EventMeta {
        EventMeta {
            slug: slug.to_string(),
            title: title.to_string(),
            start: start.to_string(),
            end: end.to_string(),
            description: None,
            item_count: 0,
            sections: Vec::new(),
        }
    }

    fn event_catalog(items: Vec<CatalogItemInput>, events: Vec<EventMeta>) -> Catalog {
        let mut catalog = build_catalog(items, "/media", "/thumbs");
        crate::events::attach_events(&mut catalog, events).unwrap();
        catalog
    }

    fn cross_month_catalog() -> Catalog {
        event_catalog(
            vec![
                item("2024/05/a.jpg", 2024, 5, "2024-05-30 10:00:00"),
                item("2024/06/b.jpg", 2024, 6, "2024-06-02 10:00:00"),
            ],
            vec![EventMeta {
                slug: "may-trip".into(),
                title: "May trip".into(),
                start: "2024-05-28 09:00".into(),
                end: "2024-06-05 18:30".into(),
                description: Some("A crossing event.".into()),
                item_count: 0,
                sections: Vec::new(),
            }],
        )
    }

    #[test]
    fn root_and_event_list_show_events_above_years() {
        let catalog = cross_month_catalog();
        let root = render_index(&catalog).unwrap();
        let events_heading = root.find("<h1>Events</h1>").expect("events heading");
        let years_heading = root.find("<h1>Years</h1>").expect("years heading");
        assert!(events_heading < years_heading);
        assert!(root.contains("href=\"/events/may-trip/\""));
        assert!(root.contains("May trip"));

        let list = render_events(&catalog).unwrap();
        assert!(list.contains("May trip"));
        assert!(list.contains("2024-05-28 09:00 to 2024-06-05 18:30"));
        assert!(list.contains("2 items"));
        assert!(list.contains("href=\"/events/may-trip/\""));
        assert!(list.contains("A crossing event."));
    }

    #[test]
    fn empty_event_list_renders_without_error() {
        let catalog = build_catalog(vec![], "/media", "/thumbs");
        let html = render_events(&catalog).unwrap();
        assert!(html.contains("No events configured."));
    }

    #[test]
    fn snapshot_event_page() {
        let html = render_event(&cross_month_catalog(), "may-trip")
            .unwrap()
            .unwrap();
        insta::assert_snapshot!("event_may_trip", html);
    }

    fn section(slug: &str, title: &str, start: &str, end: &str) -> EventSection {
        EventSection {
            slug: slug.to_string(),
            title: title.to_string(),
            start: start.to_string(),
            end: end.to_string(),
            description: None,
        }
    }

    fn sectioned_catalog() -> Catalog {
        event_catalog(
            vec![
                item("2024/05/lisbon.jpg", 2024, 5, "2024-05-02 10:00:00"),
                item("2024/05/algarve.jpg", 2024, 5, "2024-05-10 10:00:00"),
            ],
            vec![EventMeta {
                slug: "portugal".into(),
                title: "Portugal".into(),
                start: "2024-05-01 00:00".into(),
                end: "2024-05-31 23:59".into(),
                description: Some("Five weeks in Portugal.".into()),
                item_count: 0,
                sections: vec![
                    section("lisbon", "Lisbon", "2024-05-01 00:00", "2024-05-09 23:59"),
                    section("algarve", "Algarve", "2024-05-10 00:00", "2024-05-31 23:59"),
                ],
            }],
        )
    }

    #[test]
    fn snapshot_sectioned_event_page() {
        let html = render_event(&sectioned_catalog(), "portugal")
            .unwrap()
            .unwrap();
        insta::assert_snapshot!("event_portugal_sections", html);
    }

    #[test]
    fn sectioned_event_page_navigates_and_groups_items_chronologically() {
        let html = render_event(&sectioned_catalog(), "portugal")
            .unwrap()
            .unwrap();
        assert!(html.contains("<nav class=\"section-nav\" aria-label=\"Sections\">"));
        assert!(html.contains("href=\"#lisbon\""));
        assert!(html.contains("href=\"#algarve\""));
        assert!(html.contains("id=\"lisbon\""));
        assert!(html.contains("id=\"algarve\""));
        assert!(html.contains("2024-05-01 00:00 to 2024-05-09 23:59"));
        assert!(html.contains("2024-05-10 00:00 to 2024-05-31 23:59"));
        assert!(!html.contains("class=\"month-section\""));

        let lisbon_nav = html.find("href=\"#lisbon\"").unwrap();
        let algarve_nav = html.find("href=\"#algarve\"").unwrap();
        assert!(lisbon_nav < algarve_nav);

        let lisbon_item = html.find("href=\"/media/2024/05/lisbon.jpg\"").unwrap();
        let algarve_item = html.find("href=\"/media/2024/05/algarve.jpg\"").unwrap();
        let lisbon_heading = html.find("id=\"lisbon\"").unwrap();
        let algarve_heading = html.find("id=\"algarve\"").unwrap();
        assert!(lisbon_heading < lisbon_item && lisbon_item < algarve_heading);
        assert!(algarve_heading < algarve_item);
        assert_eq!(html.matches("/assets/viewer.js").count(), 1);
        assert!(!html.contains("<script>"));
    }

    #[test]
    fn empty_sections_are_omitted_from_content_and_navigation() {
        let catalog = event_catalog(
            vec![item("2024/05/algarve.jpg", 2024, 5, "2024-05-10 10:00:00")],
            vec![EventMeta {
                slug: "portugal".into(),
                title: "Portugal".into(),
                start: "2024-05-01 00:00".into(),
                end: "2024-05-31 23:59".into(),
                description: None,
                item_count: 0,
                sections: vec![
                    section("lisbon", "Lisbon", "2024-05-01 00:00", "2024-05-09 23:59"),
                    section("algarve", "Algarve", "2024-05-10 00:00", "2024-05-31 23:59"),
                ],
            }],
        );
        let html = render_event(&catalog, "portugal").unwrap().unwrap();
        assert!(!html.contains("id=\"lisbon\""));
        assert!(!html.contains("href=\"#lisbon\""));
        assert!(html.contains("id=\"algarve\""));
        assert!(html.contains("href=\"#algarve\""));
    }

    #[test]
    fn other_bucket_renders_items_outside_published_sections() {
        let catalog = event_catalog(
            vec![
                item("2024/05/before.jpg", 2024, 5, "2024-05-01 00:00:00"),
                item("2024/05/inside.jpg", 2024, 5, "2024-05-10 10:00:00"),
                item("2024/05/gap.jpg", 2024, 5, "2024-05-20 10:00:00"),
            ],
            vec![EventMeta {
                slug: "portugal".into(),
                title: "Portugal".into(),
                start: "2024-05-01 00:00".into(),
                end: "2024-05-31 23:59".into(),
                description: None,
                item_count: 0,
                sections: vec![section(
                    "lisbon",
                    "Lisbon",
                    "2024-05-05 00:00",
                    "2024-05-15 23:59",
                )],
            }],
        );
        let html = render_event(&catalog, "portugal").unwrap().unwrap();
        assert!(html.contains("id=\"other\""));
        let other = html.find("id=\"other\"").unwrap();
        assert!(html.find("href=\"/media/2024/05/before.jpg\"").unwrap() > other);
        assert!(html.find("href=\"/media/2024/05/gap.jpg\"").unwrap() > other);
        assert!(html.find("href=\"/media/2024/05/inside.jpg\"").unwrap() > other);

        let covered = event_catalog(
            vec![item("2024/05/inside.jpg", 2024, 5, "2024-05-10 10:00:00")],
            vec![EventMeta {
                slug: "portugal".into(),
                title: "Portugal".into(),
                start: "2024-05-01 00:00".into(),
                end: "2024-05-31 23:59".into(),
                description: None,
                item_count: 0,
                sections: vec![section(
                    "lisbon",
                    "Lisbon",
                    "2024-05-05 00:00",
                    "2024-05-15 23:59",
                )],
            }],
        );
        let html = render_event(&covered, "portugal").unwrap().unwrap();
        assert!(!html.contains("id=\"other\""));
    }

    #[test]
    fn configured_sections_replace_month_groups_even_when_all_sections_are_empty() {
        let catalog = event_catalog(
            vec![item("2024/05/gap.jpg", 2024, 5, "2024-05-20 10:00:00")],
            vec![EventMeta {
                slug: "portugal".into(),
                title: "Portugal".into(),
                start: "2024-05-01 00:00".into(),
                end: "2024-05-31 23:59".into(),
                description: None,
                item_count: 0,
                sections: vec![section(
                    "lisbon",
                    "Lisbon",
                    "2024-05-01 00:00",
                    "2024-05-09 23:59",
                )],
            }],
        );
        let html = render_event(&catalog, "portugal").unwrap().unwrap();
        assert!(!html.contains("class=\"month-section\""));
        assert!(html.contains("class=\"section-nav\""));
        assert!(html.contains("href=\"#other\""));
        assert!(!html.contains("href=\"#lisbon\""));
        assert!(html.contains("id=\"other\""));
        assert!(html.contains("href=\"/media/2024/05/gap.jpg\""));
    }

    #[test]
    fn event_page_groups_months_newest_first_and_links_month_pages() {
        let catalog = cross_month_catalog();
        let html = render_event(&catalog, "may-trip").unwrap().unwrap();
        assert!(html.contains("<h1>May trip</h1>"));
        assert!(html.contains("2024-05-28 09:00 to 2024-06-05 18:30"));
        assert!(html.contains("A crossing event."));

        let june = html.find("June 2024").expect("June heading");
        let may = html.find("May 2024").expect("May heading");
        assert!(june < may);
        assert!(html.contains("href=\"/2024/06/\""));
        assert!(html.contains("href=\"/2024/05/\""));

        let june_item = html
            .find("href=\"/media/2024/06/b.jpg\"")
            .expect("June item");
        let may_item = html
            .find("href=\"/media/2024/05/a.jpg\"")
            .expect("May item");
        assert!(june_item < may_item);
        assert_eq!(html.matches("loading=\"lazy\"").count(), 2);
        assert_eq!(html.matches("/assets/viewer.js").count(), 1);
        assert!(!html.contains("<script>"));
    }

    #[test]
    fn event_page_reuses_tile_presentation_and_escapes_metadata() {
        let catalog = event_catalog(
            vec![
                CatalogItemInput {
                    kind: MediaKind::Video,
                    duration_ms: Some(65_000),
                    ..item("2024/05/clip.mp4", 2024, 5, "2024-05-30 10:00:00")
                },
                CatalogItemInput {
                    thumb_available: false,
                    thumb_version: None,
                    ..item(
                        "2024/05/<script>alert(1).jpg",
                        2024,
                        5,
                        "2024-05-30 11:00:00",
                    )
                },
            ],
            vec![event_meta(
                "trip",
                "Trip",
                "2024-05-01 00:00",
                "2024-05-31 23:59",
            )],
        );
        let html = render_event(&catalog, "trip").unwrap().unwrap();
        assert!(html.contains("Video"));
        assert!(html.contains("1:05"));
        assert!(html.contains("class=\"placeholder\""));
        assert!(!html.contains("<script>alert(1)"));
        assert!(html.contains("href=\"/media/2024/05/%3Cscript%3Ealert(1).jpg\""));
    }

    #[test]
    fn empty_event_renders_a_summary_and_empty_state() {
        let catalog = event_catalog(
            vec![item("2024/05/a.jpg", 2024, 5, "2024-05-01 10:00:00")],
            vec![event_meta(
                "empty",
                "Empty trip",
                "2024-06-01 00:00",
                "2024-06-30 23:59",
            )],
        );
        let html = render_event(&catalog, "empty").unwrap().unwrap();
        assert!(html.contains("<h1>Empty trip</h1>"));
        assert!(html.contains("No items fall within this event."));
    }

    #[test]
    fn unknown_event_slug_has_no_page() {
        let catalog = cross_month_catalog();
        assert!(render_event(&catalog, "missing").unwrap().is_none());
    }
}
