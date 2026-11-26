use anyhow::{Context, Result};
use askama::Template;
use chrono::{Datelike, NaiveDate, NaiveDateTime};

use crate::model::{Catalog, EventView, ItemView, MonthView};
use crate::paths::{event_page_url, month_page_url, year_page_url};
use crate::scan::MediaKind;

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    years: Vec<YearLink>,
    events: Vec<EventLink>,
    recent: Vec<TileView>,
    total_items: usize,
    location_preview: bool,
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
    location_preview: bool,
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
    prev_month: Option<MonthNavLink>,
    next_month: Option<MonthNavLink>,
    location_preview: bool,
}

/// A neighbouring month page link used by the month navigation fragment.
struct MonthNavLink {
    label: String,
    url: String,
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
    f_number: Option<f32>,
    exposure: Option<String>,
    iso: Option<u32>,
    focal_length_mm: Option<f32>,
    latitude: Option<f64>,
    longitude: Option<f64>,
}

#[derive(Template)]
#[template(path = "search.html")]
struct SearchTemplate;

/// Number of recent items shown on the root page.
const RECENT_LIMIT: usize = 24;

/// Render the root years index, including events and recent items when present.
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
        recent: recent_items(catalog)
            .into_iter()
            .map(tile_from_item)
            .collect(),
        total_items: catalog.item_count,
        location_preview: catalog.location_preview,
    }
    .render()
    .context("rendering root index")
}

/// Select the newest catalog items across every folder.
///
/// Folder order cannot answer "most recent" because capture timestamps may
/// disagree with the folder position. Ties resolve by descending path so the
/// newest-first order stays deterministic.
fn recent_items(catalog: &Catalog) -> Vec<&ItemView> {
    let mut items: Vec<&ItemView> = catalog
        .years
        .iter()
        .flat_map(|year| year.months.iter())
        .flat_map(|month| month.items.iter())
        .collect();
    items.sort_by(|left, right| {
        right
            .captured_at
            .cmp(&left.captured_at)
            .then_with(|| right.path.cmp(&left.path))
    });
    items.truncate(RECENT_LIMIT);
    items
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
        location_preview: catalog.location_preview,
    }
    .render()
    .with_context(|| format!("rendering event {}", event.slug))?;
    Ok(Some(html))
}

fn event_link(event: &EventView) -> EventLink {
    EventLink {
        title: event.title.clone(),
        range: event_span(&event.start, &event.end),
        count: event.item_count,
        url: event_page_url(&event.slug),
    }
}

/// Format a month-level span for overview entries. Days and times are dropped
/// so short events read as the month they occupy.
fn event_span(start: &str, end: &str) -> String {
    let (Some((start_year, start_month)), Some((end_year, end_month))) =
        (parse_month(start), parse_month(end))
    else {
        return format!("{start} to {end}");
    };
    let start_label = short_month(start_year, start_month);
    let end_label = short_month(end_year, end_month);
    if start_year == end_year {
        if start_month == end_month {
            format!("{start_label} {start_year}")
        } else {
            format!("{start_label}-{end_label} {start_year}")
        }
    } else {
        format!("{start_label} {start_year} - {end_label} {end_year}")
    }
}

fn parse_month(raw: &str) -> Option<(i32, u32)> {
    NaiveDateTime::parse_from_str(raw.trim(), "%Y-%m-%d %H:%M")
        .ok()
        .map(|timestamp| (timestamp.year(), timestamp.month()))
}

fn short_month(year: i32, month: u32) -> String {
    NaiveDate::from_ymd_opt(year, month, 1)
        .map(|date| date.format("%b").to_string())
        .unwrap_or_else(|| format!("{month:02}"))
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
    let (prev_month, next_month) = month_neighbours(catalog, view.year, view.month);
    let html = MonthTemplate {
        year: view.year,
        month_label: view.label.clone(),
        year_url: year_page_url(view.year),
        item_count: view.items.len(),
        items: view.items.iter().map(tile_from_item).collect(),
        prev_month,
        next_month,
        location_preview: catalog.location_preview,
    }
    .render()
    .with_context(|| format!("rendering {}-{:02}", view.year, view.month))?;
    Ok(Some(html))
}

/// Find the nearest earlier and later month pages in chronological order.
///
/// Month pages are sparse, so adjacency skips missing months and crosses year
/// boundaries. A direction is `None` when no page exists on that side.
fn month_neighbours(
    catalog: &Catalog,
    year: i32,
    month: u32,
) -> (Option<MonthNavLink>, Option<MonthNavLink>) {
    let mut pages: Vec<&MonthView> = catalog
        .years
        .iter()
        .flat_map(|year| year.months.iter())
        .collect();
    pages.sort_by_key(|view| (view.year, view.month));
    let Some(index) = pages
        .iter()
        .position(|view| view.year == year && view.month == month)
    else {
        return (None, None);
    };
    let nav_link = |view: &MonthView| MonthNavLink {
        label: view.label.clone(),
        url: month_page_url(view.year, view.month),
    };
    let prev = index.checked_sub(1).map(|earlier| nav_link(pages[earlier]));
    let next = pages.get(index + 1).map(|later| nav_link(later));
    (prev, next)
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
        f_number: item.f_number,
        exposure: item.exposure.map(|exposure| exposure.fraction()),
        iso: item.iso,
        focal_length_mm: item.focal_length_mm,
        latitude: item.latitude,
        longitude: item.longitude,
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
            ..CatalogItemInput::default()
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
    fn recent_items_are_newest_first_across_folders() {
        let catalog = build_catalog(
            vec![
                item("2024/01/newer.jpg", 2024, 1, "2024-06-30 10:00:00"),
                item("2024/06/folder-newest.jpg", 2024, 6, "2024-06-01 10:00:00"),
                item("2023/12/older.jpg", 2023, 12, "2023-12-01 10:00:00"),
                item("2024/06/tie-b.jpg", 2024, 6, "2024-06-15 10:00:00"),
                item("2024/05/tie-a.jpg", 2024, 5, "2024-06-15 10:00:00"),
            ],
            "/media",
            "/thumbs",
        );

        let paths: Vec<_> = recent_items(&catalog)
            .iter()
            .map(|item| item.path.as_str())
            .collect();
        assert_eq!(
            paths,
            vec![
                "2024/01/newer.jpg",
                "2024/06/tie-b.jpg",
                "2024/05/tie-a.jpg",
                "2024/06/folder-newest.jpg",
                "2023/12/older.jpg",
            ]
        );
    }

    #[test]
    fn recent_selection_is_empty_safe_and_capped() {
        let empty = build_catalog(vec![], "/media", "/thumbs");
        assert!(recent_items(&empty).is_empty());

        let few = build_catalog(
            vec![
                item("2024/05/a.jpg", 2024, 5, "2024-05-01 10:00:00"),
                item("2024/05/b.jpg", 2024, 5, "2024-05-02 10:00:00"),
                item("2024/05/c.jpg", 2024, 5, "2024-05-03 10:00:00"),
            ],
            "/media",
            "/thumbs",
        );
        assert_eq!(recent_items(&few).len(), 3);

        let many = build_catalog(
            (0..30)
                .map(|index| {
                    item(
                        &format!("2024/05/item-{index:03}.jpg"),
                        2024,
                        5,
                        "2024-05-01 10:00:00",
                    )
                })
                .collect(),
            "/media",
            "/thumbs",
        );
        let recent = recent_items(&many);
        assert_eq!(recent.len(), RECENT_LIMIT);
        assert_eq!(recent.first().unwrap().path, "2024/05/item-029.jpg");
        assert_eq!(recent.last().unwrap().path, "2024/05/item-006.jpg");
    }

    #[test]
    fn recent_tiles_keep_video_and_placeholder_presentation() {
        let catalog = build_catalog(
            vec![
                CatalogItemInput {
                    kind: MediaKind::Video,
                    duration_ms: Some(65_000),
                    ..item("2024/05/clip.mp4", 2024, 5, "2024-05-02 10:00:00")
                },
                CatalogItemInput {
                    thumb_available: false,
                    thumb_version: None,
                    ..item("2024/05/a.heic", 2024, 5, "2024-05-01 10:00:00")
                },
            ],
            "/media",
            "/thumbs",
        );
        let html = render_index(&catalog).unwrap();
        assert!(html.contains("<h1>Recent</h1>"));
        assert!(html.contains("class=\"grid recent-grid\""));
        assert!(html.contains("href=\"/media/2024/05/clip.mp4\""));
        assert!(html.contains("data-kind=\"video\""));
        assert!(html.contains("Video"));
        assert!(html.contains("1:05"));
        assert!(html.contains("class=\"placeholder\""));
        assert!(html.contains("aria-label=\"a.heic\""));
    }

    #[test]
    fn archive_pages_list_ascending_months_and_oldest_items() {
        let catalog = build_catalog(
            vec![
                item("2024/12/late.jpg", 2024, 12, "2024-12-02 10:00:00"),
                item("2024/05/later.jpg", 2024, 5, "2024-05-02 10:00:00"),
                item("2024/05/early.jpg", 2024, 5, "2024-05-01 10:00:00"),
                item("2024/01/first.jpg", 2024, 1, "2024-01-01 10:00:00"),
            ],
            "/media",
            "/thumbs",
        );

        let year = render_year(&catalog, 2024).unwrap().unwrap();
        let january = year.find("href=\"/2024/01/\"").expect("January link");
        let may = year.find("href=\"/2024/05/\"").expect("May link");
        let december = year.find("href=\"/2024/12/\"").expect("December link");
        assert!(january < may && may < december, "year months: {year}");

        let month = render_month(&catalog, 2024, 5).unwrap().unwrap();
        let early = month
            .find("href=\"/media/2024/05/early.jpg\"")
            .expect("early item");
        let later = month
            .find("href=\"/media/2024/05/later.jpg\"")
            .expect("later item");
        assert!(early < later, "month items: {month}");

        insta::assert_snapshot!("year_2024_ordered_months", year);
        insta::assert_snapshot!("month_2024_05_ordered_items", month);
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
    fn month_navigation_links_adjacent_pages_in_the_same_year() {
        let catalog = build_catalog(
            vec![
                item("2024/04/a.jpg", 2024, 4, "2024-04-01 10:00:00"),
                item("2024/05/a.jpg", 2024, 5, "2024-05-01 10:00:00"),
                item("2024/06/a.jpg", 2024, 6, "2024-06-01 10:00:00"),
            ],
            "/media",
            "/thumbs",
        );
        let html = render_month(&catalog, 2024, 5).unwrap().unwrap();
        assert!(
            html.contains("<a class=\"month-nav-prev\" href=\"/2024/04/\" rel=\"prev\">"),
            "expected previous link to April 2024, got:\n{html}"
        );
        assert!(html.contains("April 2024"));
        assert!(
            html.contains("<a class=\"month-nav-next\" href=\"/2024/06/\" rel=\"next\">"),
            "expected next link to June 2024, got:\n{html}"
        );
        assert!(html.contains("June 2024"));
    }

    #[test]
    fn month_navigation_skips_months_without_pages() {
        let catalog = build_catalog(
            vec![
                item("2024/03/a.jpg", 2024, 3, "2024-03-01 10:00:00"),
                item("2024/05/a.jpg", 2024, 5, "2024-05-01 10:00:00"),
                item("2024/07/a.jpg", 2024, 7, "2024-07-01 10:00:00"),
            ],
            "/media",
            "/thumbs",
        );
        let html = render_month(&catalog, 2024, 5).unwrap().unwrap();
        assert!(html.contains("href=\"/2024/03/\""));
        assert!(html.contains("March 2024"));
        assert!(html.contains("href=\"/2024/07/\""));
        assert!(html.contains("July 2024"));
        assert!(!html.contains("/2024/04/"));
        assert!(!html.contains("/2024/06/"));
    }

    #[test]
    fn month_navigation_crosses_year_boundaries() {
        let catalog = build_catalog(
            vec![
                item("2023/12/a.jpg", 2023, 12, "2023-12-01 10:00:00"),
                item("2024/01/b.jpg", 2024, 1, "2024-01-01 10:00:00"),
            ],
            "/media",
            "/thumbs",
        );
        let january = render_month(&catalog, 2024, 1).unwrap().unwrap();
        assert!(january.contains("<a class=\"month-nav-prev\" href=\"/2023/12/\" rel=\"prev\">"));
        assert!(january.contains("December 2023"));
        assert_eq!(january.matches("rel=\"next\"").count(), 0);

        let december = render_month(&catalog, 2023, 12).unwrap().unwrap();
        assert!(december.contains("<a class=\"month-nav-next\" href=\"/2024/01/\" rel=\"next\">"));
        assert!(december.contains("January 2024"));
        assert_eq!(december.matches("rel=\"prev\"").count(), 0);
    }

    #[test]
    fn month_navigation_omits_the_earliest_and_latest_edges() {
        let catalog = build_catalog(
            vec![
                item("2024/03/a.jpg", 2024, 3, "2024-03-01 10:00:00"),
                item("2024/05/b.jpg", 2024, 5, "2024-05-01 10:00:00"),
                item("2024/07/c.jpg", 2024, 7, "2024-07-01 10:00:00"),
            ],
            "/media",
            "/thumbs",
        );
        let earliest = render_month(&catalog, 2024, 3).unwrap().unwrap();
        assert_eq!(earliest.matches("rel=\"prev\"").count(), 0);
        assert_eq!(earliest.matches("rel=\"next\"").count(), 2);

        let latest = render_month(&catalog, 2024, 7).unwrap().unwrap();
        assert_eq!(latest.matches("rel=\"prev\"").count(), 2);
        assert_eq!(latest.matches("rel=\"next\"").count(), 0);
    }

    #[test]
    fn single_month_page_has_no_neighbour_links() {
        let catalog = build_catalog(
            vec![item("2024/05/a.jpg", 2024, 5, "2024-05-01 10:00:00")],
            "/media",
            "/thumbs",
        );
        let html = render_month(&catalog, 2024, 5).unwrap().unwrap();
        assert_eq!(html.matches("rel=\"prev\"").count(), 0);
        assert_eq!(html.matches("rel=\"next\"").count(), 0);
    }

    #[test]
    fn month_navigation_is_repeated_above_and_below_the_grid() {
        let catalog = build_catalog(
            vec![
                item("2024/04/a.jpg", 2024, 4, "2024-04-01 10:00:00"),
                item("2024/05/b.jpg", 2024, 5, "2024-05-01 10:00:00"),
                item("2024/06/c.jpg", 2024, 6, "2024-06-01 10:00:00"),
            ],
            "/media",
            "/thumbs",
        );
        let html = render_month(&catalog, 2024, 5).unwrap().unwrap();
        assert_eq!(html.matches("class=\"month-nav\"").count(), 2);
        assert_eq!(html.matches("rel=\"prev\"").count(), 2);
        assert_eq!(html.matches("rel=\"next\"").count(), 2);
        assert_eq!(html.matches("href=\"/2024/04/\"").count(), 2);
        assert_eq!(html.matches("href=\"/2024/06/\"").count(), 2);

        let first = html.find("class=\"month-nav\"").unwrap();
        let heading = html.find("<h1>").unwrap();
        let last = html.rfind("class=\"month-nav\"").unwrap();
        let grid_end = html.find("</div>").unwrap();
        assert!(first < heading, "top navigation must precede the heading");
        assert!(grid_end < last, "bottom navigation must follow the grid");
    }

    #[test]
    fn snapshot_multi_year_month_navigation() {
        let catalog = build_catalog(
            vec![
                item("2023/11/a.jpg", 2023, 11, "2023-11-01 10:00:00"),
                item("2023/12/b.jpg", 2023, 12, "2023-12-01 10:00:00"),
                item("2024/02/c.jpg", 2024, 2, "2024-02-01 10:00:00"),
            ],
            "/media",
            "/thumbs",
        );
        insta::assert_snapshot!(
            "month_2023_12_navigation",
            render_month(&catalog, 2023, 12).unwrap().unwrap()
        );
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
        assert!(
            !html.contains("3.4 MB"),
            "tile captions do not show the size"
        );
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
                CatalogItemInput {
                    f_number: Some(1.8),
                    exposure: crate::probe::Exposure::new(1, 125),
                    iso: Some(100),
                    focal_length_mm: Some(26.0),
                    latitude: Some(52.520008),
                    longitude: Some(13.404954),
                    ..item("2024/05/tagged.jpg", 2024, 5, "2024-05-03 10:00:00")
                },
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
        assert!(month.contains("data-f-number=\"1.8\""));
        assert!(month.contains("data-exposure=\"1/125\""));
        assert!(month.contains("data-iso=\"100\""));
        assert!(month.contains("data-focal=\"26\""));
        assert!(month.contains("data-lat=\"52.520008\""));
        assert!(month.contains("data-lon=\"13.404954\""));
        assert_eq!(month.matches("data-f-number=").count(), 1);
        assert!(month.contains("data-item-path=\"2024/05/clip.mp4\""));
        assert!(month.contains("data-kind=\"video\""));

        let event = render_event(&catalog, "trip").unwrap().unwrap();
        assert!(event.contains("data-item-path=\"2024/05/clip.mp4\""));
        assert!(event.contains("data-kind=\"video\""));
        assert!(event.contains("data-width=\"4000\""));
        assert!(event.contains("data-f-number=\"1.8\""));
        assert!(event.contains("data-lat=\"52.520008\""));
    }

    #[test]
    fn pages_load_the_viewer_script_once() {
        let catalog = cross_month_catalog();
        let root = render_index(&catalog).unwrap();
        let month = render_month(&catalog, 2024, 5).unwrap().unwrap();
        let event = render_event(&catalog, "may-trip").unwrap().unwrap();
        let search = render_search_shell().unwrap();
        for (label, html) in [
            ("root", root),
            ("month", month),
            ("event", event),
            ("search", search),
        ] {
            assert_eq!(
                html.matches("/assets/viewer.js").count(),
                1,
                "{label} page must load the viewer once"
            );
        }
    }

    #[test]
    fn preview_attribute_follows_the_catalog_setting() {
        let mut catalog = cross_month_catalog();
        let root_off = render_index(&catalog).unwrap();
        let month_off = render_month(&catalog, 2024, 5).unwrap().unwrap();
        let event_off = render_event(&catalog, "may-trip").unwrap().unwrap();
        assert!(!root_off.contains("data-location-preview"));
        assert!(!month_off.contains("data-location-preview"));
        assert!(!event_off.contains("data-location-preview"));

        catalog.location_preview = true;
        let root_on = render_index(&catalog).unwrap();
        let month_on = render_month(&catalog, 2024, 5).unwrap().unwrap();
        let event_on = render_event(&catalog, "may-trip").unwrap().unwrap();
        assert!(
            root_on.contains("<body data-location-preview=\"true\">"),
            "{root_on}"
        );
        assert!(
            month_on.contains("<body data-location-preview=\"true\">"),
            "{month_on}"
        );
        assert!(
            event_on.contains("<body id=\"top\" data-location-preview=\"true\">"),
            "{event_on}"
        );
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
    fn overview_span_formats_months() {
        assert_eq!(
            event_span("2024-05-01 00:00", "2024-05-31 23:59"),
            "May 2024"
        );
        assert_eq!(
            event_span("2024-05-28 09:00", "2024-06-05 18:30"),
            "May-Jun 2024"
        );
        assert_eq!(
            event_span("2024-12-20 10:00", "2025-01-10 12:00"),
            "Dec 2024 - Jan 2025"
        );
        assert_eq!(
            event_span("2024-05-01 00:00", "2026-05-31 23:59"),
            "May 2024 - May 2026"
        );
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
        assert!(root.contains("May-Jun 2024"));
        assert!(root.contains("2 items"));
        assert!(!root.contains("A crossing event."));
        assert!(!root.contains("2024-05-28 09:00 to 2024-06-05 18:30"));

        let list = render_events(&catalog).unwrap();
        assert!(list.contains("May trip"));
        assert!(list.contains("May-Jun 2024"));
        assert!(list.contains("2 items"));
        assert!(list.contains("href=\"/events/may-trip/\""));
        assert!(!list.contains("A crossing event."));
        assert!(!list.contains("2024-05-28 09:00 to 2024-06-05 18:30"));

        let event = render_event(&catalog, "may-trip").unwrap().unwrap();
        assert!(event.contains("2024-05-28 09:00 to 2024-06-05 18:30"));
        assert!(event.contains("A crossing event."));
    }

    fn multi_span_catalog() -> Catalog {
        event_catalog(
            vec![
                item("2024/05/a.jpg", 2024, 5, "2024-05-02 10:00:00"),
                item("2024/05/b.jpg", 2024, 5, "2024-05-30 10:00:00"),
                item("2024/06/c.jpg", 2024, 6, "2024-06-02 10:00:00"),
                item("2024/12/d.jpg", 2024, 12, "2024-12-21 10:00:00"),
                item("2025/01/e.jpg", 2025, 1, "2025-01-05 10:00:00"),
            ],
            vec![
                event_meta("may", "May single", "2024-05-01 00:00", "2024-05-31 23:59"),
                event_meta(
                    "may-june",
                    "May to June",
                    "2024-05-28 09:00",
                    "2024-06-05 18:30",
                ),
                event_meta(
                    "winter",
                    "Winter crossing",
                    "2024-12-20 10:00",
                    "2025-01-10 12:00",
                ),
            ],
        )
    }

    #[test]
    fn snapshot_events_multi_span() {
        insta::assert_snapshot!(
            "events_multi_span",
            render_events(&multi_span_catalog()).unwrap()
        );
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
    fn event_page_groups_months_oldest_first_and_links_month_pages() {
        let catalog = cross_month_catalog();
        let html = render_event(&catalog, "may-trip").unwrap().unwrap();
        assert!(html.contains("<h1>May trip</h1>"));
        assert!(html.contains("2024-05-28 09:00 to 2024-06-05 18:30"));
        assert!(html.contains("A crossing event."));

        let may = html.find("May 2024").expect("May heading");
        let june = html.find("June 2024").expect("June heading");
        assert!(may < june);
        assert!(html.contains("href=\"/2024/05/\""));
        assert!(html.contains("href=\"/2024/06/\""));

        let may_item = html
            .find("href=\"/media/2024/05/a.jpg\"")
            .expect("May item");
        let june_item = html
            .find("href=\"/media/2024/06/b.jpg\"")
            .expect("June item");
        assert!(may_item < june_item);
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
    fn month_grouped_event_links_back_to_top_after_each_grid() {
        let html = render_event(&cross_month_catalog(), "may-trip")
            .unwrap()
            .unwrap();
        assert!(html.contains("<body id=\"top\">"));
        assert_eq!(
            html.matches("class=\"back-to-top\" href=\"#top\"").count(),
            2
        );
        assert!(!html.contains("<script>"));

        let may_item = html.find("href=\"/media/2024/05/a.jpg\"").unwrap();
        let june_heading = html.find("June 2024").unwrap();
        let first_link = html.find("class=\"back-to-top\"").unwrap();
        assert!(may_item < first_link && first_link < june_heading);

        let june_item = html.find("href=\"/media/2024/06/b.jpg\"").unwrap();
        let second_link = html.rfind("class=\"back-to-top\"").unwrap();
        let main_end = html.find("</main>").unwrap();
        assert!(june_item < second_link && second_link < main_end);
    }

    #[test]
    fn authored_sections_and_other_bucket_link_back_to_top() {
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
        assert_eq!(
            html.matches("class=\"back-to-top\" href=\"#top\"").count(),
            2
        );
        assert_eq!(html.matches("class=\"section-nav\"").count(), 1);
        assert!(html.contains("href=\"#lisbon\""));
        assert!(html.contains("href=\"#other\""));

        let other = html.find("id=\"other\"").unwrap();
        let other_item = html.find("href=\"/media/2024/05/gap.jpg\"").unwrap();
        let lisbon = html.find("id=\"lisbon\"").unwrap();
        let lisbon_item = html.find("href=\"/media/2024/05/inside.jpg\"").unwrap();
        let first_link = html.find("class=\"back-to-top\"").unwrap();
        let second_link = html.rfind("class=\"back-to-top\"").unwrap();
        assert!(other < other_item && other_item < first_link);
        assert!(first_link < lisbon);
        assert!(lisbon < lisbon_item && lisbon_item < second_link);
    }

    #[test]
    fn empty_event_has_a_top_anchor_but_no_back_to_top_links() {
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
        assert!(html.contains("<body id=\"top\">"));
        assert_eq!(html.matches("href=\"#top\"").count(), 0);
        assert!(html.contains("No items fall within this event."));
    }

    #[test]
    fn unknown_event_slug_has_no_page() {
        let catalog = cross_month_catalog();
        assert!(render_event(&catalog, "missing").unwrap().is_none());
    }
}
