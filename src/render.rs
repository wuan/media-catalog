use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use askama::Template;

use crate::fsutil::write_atomic;
use crate::model::{Catalog, ItemView};
use crate::paths::{month_page_rel, month_page_url, year_page_rel, year_page_url};
use crate::scan::MediaKind;

const SITE_CSS: &str = include_str!("../assets/site.css");
const SEARCH_JS: &str = include_str!("../assets/search.js");

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

/// Statistics from writing the web tree.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RenderStats {
    pub pages: usize,
    pub assets: usize,
}

/// Render all HTML pages and static assets, then remove stale year/month pages.
pub fn render_all(public_root: &Path, catalog: &Catalog) -> Result<RenderStats> {
    let mut stats = RenderStats::default();

    let index = IndexTemplate {
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
    .context("rendering root index")?;
    write_atomic(&public_root.join("index.html"), index.as_bytes())?;
    stats.pages += 1;

    let mut expected_pages = BTreeSet::new();
    expected_pages.insert(public_root.join("index.html"));

    for year in &catalog.years {
        let year_page = YearTemplate {
            year: year.year,
            total_items: year.item_count,
            months: year
                .months
                .iter()
                .map(|month| MonthLink {
                    month: month.short_label.clone(),
                    count: month.items.len(),
                    url: month_page_url(year.year, month.month),
                })
                .collect(),
        }
        .render()
        .with_context(|| format!("rendering year {}", year.year))?;
        let year_path = public_root.join(year_page_rel(year.year));
        write_atomic(&year_path, year_page.as_bytes())?;
        expected_pages.insert(year_path);
        stats.pages += 1;

        for month in &year.months {
            let month_page = MonthTemplate {
                year: year.year,
                month_label: month.label.clone(),
                year_url: year_page_url(year.year),
                item_count: month.items.len(),
                items: month.items.iter().map(tile_from_item).collect(),
            }
            .render()
            .with_context(|| format!("rendering {}-{:02}", year.year, month.month))?;
            let month_path = public_root.join(month_page_rel(year.year, month.month));
            write_atomic(&month_path, month_page.as_bytes())?;
            expected_pages.insert(month_path);
            stats.pages += 1;
        }
    }

    let search = SearchTemplate.render().context("rendering search page")?;
    write_atomic(&public_root.join("search.html"), search.as_bytes())?;
    stats.pages += 1;

    write_atomic(&public_root.join("assets/site.css"), SITE_CSS.as_bytes())?;
    write_atomic(&public_root.join("assets/search.js"), SEARCH_JS.as_bytes())?;
    stats.assets = 2;

    remove_stale_pages(public_root, &expected_pages)?;
    Ok(stats)
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

/// Remove index pages for years or months that no longer exist. Confined to
/// numeric year/month directories in the public root; never touches thumbs.
fn remove_stale_pages(public_root: &Path, expected: &BTreeSet<PathBuf>) -> Result<()> {
    let entries = match std::fs::read_dir(public_root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(
                anyhow::Error::new(error).context(format!("reading {}", public_root.display()))
            );
        }
    };

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.len() != 4 || !name.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }

        let year_index = path.join("index.html");
        if year_index.exists() && !expected.contains(&year_index) {
            std::fs::remove_file(&year_index)?;
        }

        if let Ok(children) = std::fs::read_dir(&path) {
            for child in children {
                let child = child?;
                let child_path = child.path();
                if !child.file_type()?.is_dir() {
                    continue;
                }
                let child_name = child.file_name().to_string_lossy().to_string();
                if child_name.len() != 2 || !child_name.chars().all(|c| c.is_ascii_digit()) {
                    continue;
                }
                let month_index = child_path.join("index.html");
                if month_index.exists() && !expected.contains(&month_index) {
                    std::fs::remove_file(&month_index)?;
                }
                if is_empty_dir(&child_path)? {
                    let _ = std::fs::remove_dir(&child_path);
                }
            }
        }
        if is_empty_dir(&path)? {
            let _ = std::fs::remove_dir(&path);
        }
    }
    Ok(())
}

fn is_empty_dir(path: &Path) -> Result<bool> {
    Ok(std::fs::read_dir(path)?.next().is_none())
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

    fn render(catalog: &Catalog) -> (tempfile::TempDir, PathBuf) {
        let temp = tempfile::tempdir().unwrap();
        render_all(temp.path(), catalog).unwrap();
        let public = temp.path().to_path_buf();
        (temp, public)
    }

    #[test]
    fn snapshot_empty_catalog() {
        let catalog = build_catalog(vec![], "/media", "/thumbs");
        let (temp, public) = render(&catalog);
        let html = std::fs::read_to_string(public.join("index.html")).unwrap();
        insta::assert_snapshot!("index_empty", html);
        drop(temp);
    }

    #[test]
    fn snapshot_single_year() {
        let catalog = build_catalog(
            vec![item("2024/05/a.jpg", 2024, 5, "2024-05-01 10:00:00")],
            "/media",
            "/thumbs",
        );
        let (temp, public) = render(&catalog);
        insta::assert_snapshot!(
            "index_single_year",
            std::fs::read_to_string(public.join("index.html")).unwrap()
        );
        insta::assert_snapshot!(
            "year_2024",
            std::fs::read_to_string(public.join("2024/index.html")).unwrap()
        );
        insta::assert_snapshot!(
            "month_2024_05",
            std::fs::read_to_string(public.join("2024/05/index.html")).unwrap()
        );
        drop(temp);
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
        let (temp, public) = render(&catalog);
        insta::assert_snapshot!(
            "index_multi_year",
            std::fs::read_to_string(public.join("index.html")).unwrap()
        );
        drop(temp);
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
        let (temp, public) = render(&catalog);
        let html = std::fs::read_to_string(public.join("2024/05/index.html")).unwrap();
        assert!(
            html.len() < 2_000_000,
            "month page was {} bytes",
            html.len()
        );
        assert_eq!(html.matches("href=\"/media/").count(), 500);
        assert_eq!(html.matches("loading=\"lazy\"").count(), 500);
        drop(temp);
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
        let (temp, public) = render(&catalog);
        let html = std::fs::read_to_string(public.join("2024/05/index.html")).unwrap();
        assert!(!html.contains("<script>alert(1)"));
        assert!(
            html.contains("&#60;script&#62;alert(1).jpg")
                || html.contains("&lt;script&gt;alert(1).jpg")
        );
        assert!(
            html.contains("alt=\"&#60;script&#62;alert(1).jpg\"")
                || html.contains("alt=\"&lt;script&gt;alert(1).jpg\"")
        );
        drop(temp);
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
        let (temp, public) = render(&catalog);
        let html = std::fs::read_to_string(public.join("2024/05/index.html")).unwrap();
        assert!(html.contains("class=\"placeholder\""));
        assert!(html.contains("aria-label=\"a.heic\""));
        assert!(!html.contains("<img"));
        drop(temp);
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
        let (temp, public) = render(&catalog);
        let html = std::fs::read_to_string(public.join("2024/05/index.html")).unwrap();
        assert!(html.contains("Video"));
        assert!(html.contains("1:05"));
        drop(temp);
    }

    #[test]
    fn stale_year_pages_are_removed() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(temp.path().join("2019/01")).unwrap();
        std::fs::write(temp.path().join("2019/index.html"), "old").unwrap();
        std::fs::write(temp.path().join("2019/01/index.html"), "old").unwrap();
        std::fs::write(temp.path().join("index.html"), "old").unwrap();
        let catalog = build_catalog(
            vec![item("2024/05/a.jpg", 2024, 5, "2024-05-01 10:00:00")],
            "/media",
            "/thumbs",
        );
        render_all(temp.path(), &catalog).unwrap();
        assert!(!temp.path().join("2019/index.html").exists());
        assert!(!temp.path().join("2019/01/index.html").exists());
        assert!(temp.path().join("2024/05/index.html").exists());
    }
}
