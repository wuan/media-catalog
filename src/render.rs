use anyhow::{Context, Result};
use askama::Template;

use crate::model::{Catalog, ItemView};
use crate::paths::{month_page_url, year_page_url};
use crate::scan::MediaKind;

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    years: Vec<YearLink>,
    total_items: usize,
}

struct YearLink {
    year: i32,
    count: usize,
    url: String,
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
    thumb_url: String,
    thumb_available: bool,
    alt: String,
    name: String,
    is_video: bool,
    duration_label: String,
    captured_label: String,
}

#[derive(Template)]
#[template(path = "search.html")]
struct SearchTemplate;

/// Render the root years index.
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
        total_items: catalog.item_count,
    }
    .render()
    .context("rendering root index")
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
        thumb_url: item.thumb_url.clone().unwrap_or_default(),
        thumb_available: item.thumb_available && item.thumb_url.is_some(),
        alt,
        name: item.name.clone(),
        is_video: item.kind == MediaKind::Video,
        duration_label: item.duration_ms.map(format_duration).unwrap_or_default(),
        captured_label: item.captured_at.clone(),
    }
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
    use crate::model::{build_catalog, CatalogItemInput};

    fn item(path: &str, year: i32, month: u32, captured_at: &str) -> CatalogItemInput {
        CatalogItemInput {
            rel_path: path.to_string(),
            year,
            month,
            kind: MediaKind::Image,
            captured_at: Some(captured_at.to_string()),
            mtime_ns: 0,
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
    fn escapes_markup_and_alt_text_is_non_empty() {
        let catalog = build_catalog(
            vec![item(
                "2024/05/<script>alert(1).jpg",
                2024,
                5,
                "2024-05-01 10:00:00",
            )],
            "/media",
            "/thumbs",
        );
        let html = render_month(&catalog, 2024, 5).unwrap().unwrap();
        assert!(!html.contains("<script>alert(1)"));
        assert!(
            html.contains("&#60;script&#62;alert(1).jpg")
                || html.contains("&lt;script&gt;alert(1).jpg")
        );
        assert!(
            html.contains("alt=\"&#60;script&#62;alert(1).jpg\"")
                || html.contains("alt=\"&lt;script&gt;alert(1).jpg\"")
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
}
