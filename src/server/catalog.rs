use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::UNIX_EPOCH;

use anyhow::{bail, Context, Result};
use askama::Template;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{header, HeaderMap, HeaderValue, Method, StatusCode};
use axum::response::Response;
use tokio::sync::Mutex;

use super::support::{error_response, is_not_modified, method_not_allowed, not_modified_response};
use super::AppState;
use crate::json;
use crate::model::Catalog;
use crate::render;

/// Identity of the published catalog index used to detect changes and build
/// validators for rendered pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnapshotId {
    len: u64,
    mtime_ns: u128,
}

impl SnapshotId {
    fn token(self) -> String {
        format!("{:x}-{:x}", self.len, self.mtime_ns)
    }
}

/// An immutable catalog snapshot loaded from `catalog/index.json` and its shards.
#[derive(Debug)]
pub struct CatalogSnapshot {
    pub catalog: Catalog,
    pub id: SnapshotId,
}

/// Loads and caches the catalog snapshot, reloading only when the index changes.
pub struct CatalogStore {
    data_root: PathBuf,
    snapshot: Mutex<Option<Arc<CatalogSnapshot>>>,
}

impl CatalogStore {
    pub fn new(data_root: PathBuf) -> Self {
        Self {
            data_root,
            snapshot: Mutex::new(None),
        }
    }

    fn index_path(&self) -> PathBuf {
        self.data_root.join("catalog/index.json")
    }

    /// Whether a catalog index has been published. Used for readiness reporting.
    pub fn is_published(&self) -> bool {
        self.index_path().is_file()
    }

    /// Return the current snapshot, reloading it if the index metadata changed.
    /// A failed or incomplete reload keeps the previous snapshot.
    pub async fn current(&self) -> Option<Arc<CatalogSnapshot>> {
        let mut guard = self.snapshot.lock().await;

        let Some(id) = index_id(&self.index_path()) else {
            // The index is temporarily absent or unreadable. Keep serving the
            // previous complete snapshot if there is one.
            return guard.clone();
        };

        if let Some(snapshot) = guard.as_ref() {
            if snapshot.id == id {
                return Some(snapshot.clone());
            }
        }

        match load_snapshot(&self.data_root) {
            Ok(Some(snapshot)) => {
                let snapshot = Arc::new(snapshot);
                *guard = Some(snapshot.clone());
                Some(snapshot)
            }
            Ok(None) => guard.clone(),
            Err(error) => {
                eprintln!("catalog reload failed; keeping the previous snapshot: {error:#}");
                guard.clone()
            }
        }
    }
}

fn index_id(path: &std::path::Path) -> Option<SnapshotId> {
    let metadata = std::fs::metadata(path).ok()?;
    if !metadata.is_file() {
        return None;
    }
    Some(SnapshotId {
        len: metadata.len(),
        mtime_ns: metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_nanos())
            .unwrap_or(0),
    })
}

fn load_snapshot(data_root: &std::path::Path) -> Result<Option<CatalogSnapshot>> {
    let index_path = data_root.join("catalog/index.json");
    let catalog_dir = data_root.join("catalog");

    for _ in 0..3 {
        let Some(before) = index_id(&index_path) else {
            return Ok(None);
        };
        let index_text = std::fs::read_to_string(&index_path)
            .with_context(|| format!("reading {}", index_path.display()))?;
        let Some(after) = index_id(&index_path) else {
            continue;
        };
        if before != after {
            // The index changed while it was being read; retry with a fresh read.
            continue;
        }

        let index = json::parse_index(&index_text)?;
        let mut shards = BTreeMap::new();
        for index_year in &index.years {
            let name = json::shard_file_name(index_year.year, &index_year.shard)?;
            let path = catalog_dir.join(name);
            let text = std::fs::read_to_string(&path)
                .with_context(|| format!("reading {}", path.display()))?;
            shards.insert(index_year.year, json::parse_shard(&text)?);
        }
        let catalog = json::catalog_from_parts(&index, &shards)?;

        return Ok(Some(CatalogSnapshot { catalog, id: after }));
    }

    bail!("catalog index changed repeatedly while it was being reloaded")
}

/// Render the root years index, or an embedded bootstrap page before publication.
pub async fn serve_root(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
) -> Response {
    if !matches!(method, Method::GET | Method::HEAD) {
        return method_not_allowed();
    }

    match state.catalog.current().await {
        Some(snapshot) => match render::render_index(&snapshot.catalog) {
            Ok(html) => html_page(&method, html, "root", &snapshot, &headers),
            Err(error) => render_error(error),
        },
        None => bootstrap_page(&method),
    }
}

/// Render a year months page.
pub async fn serve_year(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Path(year): Path<String>,
) -> Response {
    if !matches!(method, Method::GET | Method::HEAD) {
        return method_not_allowed();
    }

    let Some(year) = parse_year(&year) else {
        return error_response(StatusCode::NOT_FOUND, "not found");
    };
    let Some(snapshot) = state.catalog.current().await else {
        return error_response(StatusCode::SERVICE_UNAVAILABLE, "catalog unavailable");
    };

    match render::render_year(&snapshot.catalog, year) {
        Ok(Some(html)) => html_page(&method, html, &format!("{year}"), &snapshot, &headers),
        Ok(None) => error_response(StatusCode::NOT_FOUND, "not found"),
        Err(error) => render_error(error),
    }
}

/// Render a month item grid.
pub async fn serve_month(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Path((year, month)): Path<(String, String)>,
) -> Response {
    if !matches!(method, Method::GET | Method::HEAD) {
        return method_not_allowed();
    }

    let (Some(year), Some(month)) = (parse_year(&year), parse_month(&month)) else {
        return error_response(StatusCode::NOT_FOUND, "not found");
    };
    let Some(snapshot) = state.catalog.current().await else {
        return error_response(StatusCode::SERVICE_UNAVAILABLE, "catalog unavailable");
    };

    match render::render_month(&snapshot.catalog, year, month) {
        Ok(Some(html)) => html_page(
            &method,
            html,
            &format!("{year}/{month:02}"),
            &snapshot,
            &headers,
        ),
        Ok(None) => error_response(StatusCode::NOT_FOUND, "not found"),
        Err(error) => render_error(error),
    }
}

/// Render the canonical event list. Returns `503` before publication.
pub async fn serve_events(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
) -> Response {
    if !matches!(method, Method::GET | Method::HEAD) {
        return method_not_allowed();
    }

    let Some(snapshot) = state.catalog.current().await else {
        return error_response(StatusCode::SERVICE_UNAVAILABLE, "catalog unavailable");
    };

    match render::render_events(&snapshot.catalog) {
        Ok(html) => html_page(&method, html, "events", &snapshot, &headers),
        Err(error) => render_error(error),
    }
}

/// Render one event page, or `404` for an undeclared slug.
pub async fn serve_event(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Path(slug): Path<String>,
) -> Response {
    if !matches!(method, Method::GET | Method::HEAD) {
        return method_not_allowed();
    }

    let Some(snapshot) = state.catalog.current().await else {
        return error_response(StatusCode::SERVICE_UNAVAILABLE, "catalog unavailable");
    };

    match render::render_event(&snapshot.catalog, &slug) {
        Ok(Some(html)) => html_page(
            &method,
            html,
            &format!("events/{slug}"),
            &snapshot,
            &headers,
        ),
        Ok(None) => error_response(StatusCode::NOT_FOUND, "not found"),
        Err(error) => render_error(error),
    }
}

fn parse_year(value: &str) -> Option<i32> {
    if value.len() != 4 || !value.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let year = value.parse::<i32>().ok()?;
    (1000..=2999).contains(&year).then_some(year)
}

fn parse_month(value: &str) -> Option<u32> {
    if value.len() != 2 || !value.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let month = value.parse::<u32>().ok()?;
    (1..=12).contains(&month).then_some(month)
}

fn page_etag(snapshot: &CatalogSnapshot, route: &str) -> String {
    format!("\"{}-{route}\"", snapshot.id.token())
}

fn html_page(
    method: &Method,
    html: String,
    route: &str,
    snapshot: &CatalogSnapshot,
    headers: &HeaderMap,
) -> Response {
    let etag = page_etag(snapshot, route);
    if is_not_modified(headers, &etag, None) {
        return not_modified_response(&etag, None, "no-cache");
    }
    html_response(method, html, "no-cache", Some(&etag))
}

fn html_response(
    method: &Method,
    html: String,
    cache_control: &'static str,
    etag: Option<&str>,
) -> Response {
    let length = html.len();
    let body = if *method == Method::HEAD {
        Body::empty()
    } else {
        Body::from(html)
    };
    let mut response = Response::new(body);
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    headers.insert(
        header::CONTENT_LENGTH,
        HeaderValue::from_str(&length.to_string())
            .unwrap_or_else(|_| HeaderValue::from_static("0")),
    );
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(cache_control),
    );
    if let Some(etag) = etag {
        if let Ok(value) = HeaderValue::from_str(etag) {
            headers.insert(header::ETAG, value);
        }
    }
    response
}

fn render_error(error: anyhow::Error) -> Response {
    eprintln!("rendering catalog page failed: {error:#}");
    error_response(StatusCode::INTERNAL_SERVER_ERROR, "rendering error")
}

#[derive(Template)]
#[template(path = "bootstrap.html")]
struct BootstrapTemplate;

fn bootstrap_page(method: &Method) -> Response {
    match BootstrapTemplate.render() {
        Ok(html) => html_response(method, html, "no-store", None),
        Err(error) => render_error(error.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{build_catalog, CatalogItemInput, EventMeta};
    use crate::scan::MediaKind;

    fn item(path: &str, year: i32, month: u32) -> CatalogItemInput {
        CatalogItemInput {
            rel_path: path.to_string(),
            year,
            month,
            kind: MediaKind::Image,
            captured_at: Some("2024-05-01 10:00:00".to_string()),
            mtime_ns: 0,
            width: Some(10),
            height: Some(10),
            duration_ms: None,
            thumb_version: Some("1-2".to_string()),
            thumb_available: true,
        }
    }

    fn write_catalog(data_root: &std::path::Path, catalog: &Catalog) {
        let output = crate::json::render_json(catalog).unwrap();
        let catalog_dir = data_root.join("catalog");
        std::fs::create_dir_all(&catalog_dir).unwrap();
        for (year, shard) in &output.shards {
            std::fs::write(catalog_dir.join(format!("{year}.json")), shard).unwrap();
        }
        std::fs::write(catalog_dir.join("index.json"), output.index).unwrap();
    }

    fn event_catalog(count: usize) -> Catalog {
        let items = (0..count)
            .map(|index| item(&format!("2024/05/{index}.jpg"), 2024, 5))
            .collect();
        let mut catalog = build_catalog(items, "/media", "/thumbs");
        crate::events::attach_events(
            &mut catalog,
            vec![EventMeta {
                slug: "trip".into(),
                title: "Trip".into(),
                start: "2024-05-01 00:00".into(),
                end: "2024-05-31 23:59".into(),
                description: None,
                item_count: 0,
                sections: Vec::new(),
            }],
        )
        .unwrap();
        catalog
    }

    #[tokio::test]
    async fn loads_a_snapshot_and_reloads_when_the_index_changes() {
        let temp = tempfile::tempdir().unwrap();
        let store = CatalogStore::new(temp.path().to_path_buf());
        assert!(store.current().await.is_none());

        let first = build_catalog(vec![item("2024/05/a.jpg", 2024, 5)], "/media", "/thumbs");
        write_catalog(temp.path(), &first);
        let snapshot = store.current().await.unwrap();
        assert_eq!(snapshot.catalog.item_count, 1);
        let first_id = snapshot.id;

        std::thread::sleep(std::time::Duration::from_millis(10));
        let second = build_catalog(
            vec![
                item("2024/05/a.jpg", 2024, 5),
                item("2024/05/b.jpg", 2024, 5),
            ],
            "/media",
            "/thumbs",
        );
        write_catalog(temp.path(), &second);
        let snapshot = store.current().await.unwrap();
        assert_eq!(snapshot.catalog.item_count, 2);
        assert_ne!(snapshot.id, first_id);
    }

    #[tokio::test]
    async fn keeps_the_previous_snapshot_when_a_referenced_shard_is_missing() {
        let temp = tempfile::tempdir().unwrap();
        let store = CatalogStore::new(temp.path().to_path_buf());

        let first = build_catalog(vec![item("2024/05/a.jpg", 2024, 5)], "/media", "/thumbs");
        write_catalog(temp.path(), &first);
        assert_eq!(store.current().await.unwrap().catalog.item_count, 1);

        std::thread::sleep(std::time::Duration::from_millis(10));
        let second = build_catalog(
            vec![
                item("2024/05/a.jpg", 2024, 5),
                item("2025/01/b.jpg", 2025, 1),
            ],
            "/media",
            "/thumbs",
        );
        let output = crate::json::render_json(&second).unwrap();
        let catalog_dir = temp.path().join("catalog");
        std::fs::write(catalog_dir.join("2024.json"), &output.shards[&2024]).unwrap();
        // Deliberately omit catalog/2025.json, then publish the new index.
        std::fs::write(catalog_dir.join("index.json"), &output.index).unwrap();

        let snapshot = store.current().await.unwrap();
        assert_eq!(snapshot.catalog.item_count, 1);
        assert!(snapshot.catalog.year(2025).is_none());
    }

    fn sectioned_event_catalog(count: usize) -> Catalog {
        let items = (0..count)
            .map(|index| item(&format!("2024/05/{index}.jpg"), 2024, 5))
            .collect();
        let mut catalog = build_catalog(items, "/media", "/thumbs");
        crate::events::attach_events(
            &mut catalog,
            vec![EventMeta {
                slug: "trip".into(),
                title: "Trip".into(),
                start: "2024-05-01 00:00".into(),
                end: "2024-05-31 23:59".into(),
                description: None,
                item_count: 0,
                sections: vec![
                    crate::model::EventSection {
                        slug: "first".into(),
                        title: "First".into(),
                        start: "2024-05-01 00:00".into(),
                        end: "2024-05-09 23:59".into(),
                        description: None,
                    },
                    crate::model::EventSection {
                        slug: "second".into(),
                        title: "Second".into(),
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

    #[tokio::test]
    async fn keeps_the_previous_snapshot_when_published_sections_are_invalid() {
        let temp = tempfile::tempdir().unwrap();
        let store = CatalogStore::new(temp.path().to_path_buf());

        write_catalog(temp.path(), &sectioned_event_catalog(2));
        let snapshot = store.current().await.unwrap();
        assert_eq!(snapshot.catalog.events[0].sections.len(), 2);
        let first_id = snapshot.id;

        std::thread::sleep(std::time::Duration::from_millis(10));
        let output = crate::json::render_json(&sectioned_event_catalog(3)).unwrap();
        let mut index = crate::json::parse_index(&output.index).unwrap();
        index.events[0].sections[1].start = "2024-05-09 23:59".into();
        let catalog_dir = temp.path().join("catalog");
        for (year, shard) in &output.shards {
            std::fs::write(catalog_dir.join(format!("{year}.json")), shard).unwrap();
        }
        std::fs::write(
            catalog_dir.join("index.json"),
            serde_json::to_string_pretty(&index).unwrap(),
        )
        .unwrap();

        let snapshot = store.current().await.unwrap();
        assert_eq!(snapshot.id, first_id);
        assert_eq!(snapshot.catalog.item_count, 2);
        assert_eq!(snapshot.catalog.events[0].sections.len(), 2);
    }

    #[tokio::test]
    async fn keeps_the_previous_snapshot_when_event_data_is_inconsistent() {
        let temp = tempfile::tempdir().unwrap();
        let store = CatalogStore::new(temp.path().to_path_buf());

        write_catalog(temp.path(), &event_catalog(2));
        let snapshot = store.current().await.unwrap();
        assert_eq!(snapshot.catalog.item_count, 2);
        assert_eq!(snapshot.catalog.events.len(), 1);
        let first_id = snapshot.id;

        std::thread::sleep(std::time::Duration::from_millis(10));
        let second = event_catalog(3);
        let output = crate::json::render_json(&second).unwrap();
        let mut index = crate::json::parse_index(&output.index).unwrap();
        index.events[0].count += 1;
        let catalog_dir = temp.path().join("catalog");
        for (year, shard) in &output.shards {
            std::fs::write(catalog_dir.join(format!("{year}.json")), shard).unwrap();
        }
        std::fs::write(
            catalog_dir.join("index.json"),
            serde_json::to_string_pretty(&index).unwrap(),
        )
        .unwrap();

        let snapshot = store.current().await.unwrap();
        assert_eq!(snapshot.id, first_id);
        assert_eq!(snapshot.catalog.item_count, 2);
        assert_eq!(snapshot.catalog.events[0].item_count, 2);
    }
}
