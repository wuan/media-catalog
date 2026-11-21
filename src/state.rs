use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use rusqlite::{params, Connection, OpenFlags, OptionalExtension};

use crate::scan::{MediaKind, ScannedFile};

pub const SCHEMA_VERSION: i64 = 1;

/// Thumbnail lifecycle state stored per item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThumbStatus {
    Pending,
    Ok,
    Failed,
    Unsupported,
}

impl ThumbStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            ThumbStatus::Pending => "pending",
            ThumbStatus::Ok => "ok",
            ThumbStatus::Failed => "failed",
            ThumbStatus::Unsupported => "unsupported",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(ThumbStatus::Pending),
            "ok" => Some(ThumbStatus::Ok),
            "failed" => Some(ThumbStatus::Failed),
            "unsupported" => Some(ThumbStatus::Unsupported),
            _ => None,
        }
    }
}

/// A persisted item row.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemRecord {
    pub path: String,
    pub year: i32,
    pub month: u32,
    pub kind: MediaKind,
    pub size: u64,
    pub mtime: i64,
    pub fingerprint: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration_ms: Option<u64>,
    pub captured_at: Option<String>,
    pub thumb_path: Option<String>,
    pub thumb_version: Option<String>,
    pub thumb_status: ThumbStatus,
    pub error: Option<String>,
    pub last_seen_run: i64,
}

/// SQLite-backed build state.
#[derive(Debug)]
pub struct StateDb {
    conn: Connection,
}

impl StateDb {
    /// Open (creating if needed) the state database and initialize the schema.
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating state directory {}", parent.display()))?;
        }
        let conn = Connection::open(path)
            .with_context(|| format!("opening state database {}", path.display()))?;
        configure(&conn)?;
        init_schema(&conn)?;
        Ok(Self { conn })
    }

    /// Open an existing state database without modifying it (used by --dry-run).
    pub fn open_read_only(path: &Path) -> Result<Self> {
        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .with_context(|| format!("opening state database {}", path.display()))?;
        let _ = conn.busy_timeout(Duration::from_secs(5));
        let version: Option<String> = conn
            .query_row(
                "SELECT value FROM meta WHERE key = 'schema_version'",
                [],
                |row| row.get(0),
            )
            .optional()
            .context("reading state schema version")?;
        match version {
            Some(version) if version == SCHEMA_VERSION.to_string() => {}
            Some(version) => bail!(
                "state database schema version {version} is unsupported (expected {SCHEMA_VERSION})"
            ),
            None => bail!("state database has no schema version; it was not created by this tool"),
        }
        Ok(Self { conn })
    }

    pub fn load_items(&self) -> Result<HashMap<String, ItemRecord>> {
        let mut statement = self.conn.prepare(
            "SELECT path, year, month, kind, size, mtime, fingerprint, width, height,
                    duration_ms, captured_at, thumb_path, thumb_version, thumb_status,
                    error, last_seen_run
             FROM items",
        )?;
        let rows = statement.query_map([], |row| {
            let kind_text: String = row.get(3)?;
            let thumb_status_text: String = row.get(13)?;
            Ok(ItemRecord {
                path: row.get(0)?,
                year: row.get(1)?,
                month: row.get(2)?,
                kind: MediaKind::parse(&kind_text).unwrap_or(MediaKind::Image),
                size: row.get::<_, i64>(4)? as u64,
                mtime: row.get(5)?,
                fingerprint: row.get(6)?,
                width: row.get(7)?,
                height: row.get(8)?,
                duration_ms: row.get::<_, Option<i64>>(9)?.map(|value| value as u64),
                captured_at: row.get(10)?,
                thumb_path: row.get(11)?,
                thumb_version: row.get(12)?,
                thumb_status: ThumbStatus::parse(&thumb_status_text)
                    .unwrap_or(ThumbStatus::Pending),
                error: row.get(14)?,
                last_seen_run: row.get(15)?,
            })
        })?;

        let mut items = HashMap::new();
        for row in rows {
            let item = row.context("reading item row")?;
            items.insert(item.path.clone(), item);
        }
        Ok(items)
    }

    /// Insert or refresh rows from a scan. Rows whose fingerprint changed reset
    /// derived metadata and thumbnail state so the pipeline regenerates them.
    pub fn upsert_scanned(&mut self, files: &[ScannedFile], run_id: i64) -> Result<()> {
        let transaction = self.conn.transaction()?;
        {
            let mut statement = transaction.prepare(
                "INSERT INTO items
                    (path, year, month, kind, size, mtime, fingerprint, thumb_status, last_seen_run)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'pending', ?8)
                 ON CONFLICT(path) DO UPDATE SET
                    year = excluded.year,
                    month = excluded.month,
                    kind = excluded.kind,
                    size = excluded.size,
                    mtime = excluded.mtime,
                    fingerprint = excluded.fingerprint,
                    last_seen_run = excluded.last_seen_run,
                    width = CASE WHEN items.fingerprint = excluded.fingerprint
                                 THEN items.width ELSE NULL END,
                    height = CASE WHEN items.fingerprint = excluded.fingerprint
                                  THEN items.height ELSE NULL END,
                    duration_ms = CASE WHEN items.fingerprint = excluded.fingerprint
                                       THEN items.duration_ms ELSE NULL END,
                    captured_at = CASE WHEN items.fingerprint = excluded.fingerprint
                                       THEN items.captured_at ELSE NULL END,
                    thumb_path = CASE WHEN items.fingerprint = excluded.fingerprint
                                      THEN items.thumb_path ELSE NULL END,
                    thumb_version = CASE WHEN items.fingerprint = excluded.fingerprint
                                         THEN items.thumb_version ELSE NULL END,
                    thumb_status = CASE WHEN items.fingerprint = excluded.fingerprint
                                        THEN items.thumb_status ELSE 'pending' END,
                    error = CASE WHEN items.fingerprint = excluded.fingerprint
                                 THEN items.error ELSE NULL END",
            )?;
            for file in files {
                statement.execute(params![
                    file.rel_path,
                    file.year,
                    file.month,
                    file.kind.as_str(),
                    file.size as i64,
                    file.mtime_ns,
                    file.fingerprint,
                    run_id,
                ])?;
            }
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn set_probe(
        &self,
        path: &str,
        width: Option<u32>,
        height: Option<u32>,
        duration_ms: Option<u64>,
        captured_at: Option<&str>,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE items SET width = ?2, height = ?3, duration_ms = ?4, captured_at = ?5
             WHERE path = ?1",
            params![
                path,
                width,
                height,
                duration_ms.map(|value| value as i64),
                captured_at
            ],
        )?;
        Ok(())
    }

    pub fn set_thumb(
        &self,
        path: &str,
        thumb_path: Option<&str>,
        thumb_version: Option<&str>,
        status: ThumbStatus,
        error: Option<&str>,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE items
             SET thumb_path = ?2, thumb_version = ?3, thumb_status = ?4, error = ?5
             WHERE path = ?1",
            params![path, thumb_path, thumb_version, status.as_str(), error],
        )?;
        Ok(())
    }

    pub fn delete_unseen(&mut self, run_id: i64) -> Result<usize> {
        let deleted = self.conn.execute(
            "DELETE FROM items WHERE last_seen_run <> ?1",
            params![run_id],
        )?;
        Ok(deleted)
    }

    pub fn item_count(&self) -> Result<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM items", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    pub fn start_run(&mut self, started_at: &str) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO runs (started_at, status) VALUES (?1, 'running')",
            params![started_at],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn finish_run(
        &self,
        run_id: i64,
        status: &str,
        item_count: usize,
        error_count: usize,
        finished_at: &str,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE runs SET finished_at = ?2, status = ?3, item_count = ?4, error_count = ?5
             WHERE id = ?1",
            params![
                run_id,
                finished_at,
                status,
                item_count as i64,
                error_count as i64
            ],
        )?;
        Ok(())
    }

    pub fn path(&self) -> Option<PathBuf> {
        self.conn.path().map(PathBuf::from)
    }
}

fn configure(conn: &Connection) -> Result<()> {
    conn.busy_timeout(Duration::from_secs(5))
        .context("setting SQLite busy timeout")?;
    conn.pragma_update(None, "journal_mode", "WAL")
        .context("enabling SQLite WAL mode")?;
    conn.pragma_update(None, "synchronous", "NORMAL")
        .context("setting SQLite synchronous mode")?;
    Ok(())
}

fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS items (
            path          TEXT PRIMARY KEY,
            year          INTEGER NOT NULL,
            month         INTEGER NOT NULL,
            kind          TEXT NOT NULL,
            size          INTEGER NOT NULL,
            mtime         INTEGER NOT NULL,
            fingerprint   TEXT NOT NULL,
            width         INTEGER,
            height        INTEGER,
            duration_ms   INTEGER,
            captured_at   TEXT,
            thumb_path    TEXT,
            thumb_version TEXT,
            thumb_status  TEXT NOT NULL,
            error         TEXT,
            last_seen_run INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS items_month ON items(year, month);

        CREATE TABLE IF NOT EXISTS runs (
            id          INTEGER PRIMARY KEY,
            started_at  TEXT,
            finished_at TEXT,
            status      TEXT,
            item_count  INTEGER,
            error_count INTEGER
        );

        CREATE TABLE IF NOT EXISTS meta (
            key   TEXT PRIMARY KEY,
            value TEXT
        );",
    )
    .context("creating state schema")?;

    let version: Option<String> = conn
        .query_row(
            "SELECT value FROM meta WHERE key = 'schema_version'",
            [],
            |row| row.get(0),
        )
        .optional()
        .context("reading state schema version")?;

    match version {
        None => {
            conn.execute(
                "INSERT INTO meta (key, value) VALUES ('schema_version', ?1)",
                params![SCHEMA_VERSION.to_string()],
            )?;
        }
        Some(version) if version == SCHEMA_VERSION.to_string() => {}
        Some(version) => bail!(
            "state database schema version {version} is unsupported (expected {SCHEMA_VERSION})"
        ),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scanned(path: &str, fingerprint: &str) -> ScannedFile {
        ScannedFile {
            rel_path: path.to_string(),
            year: 2024,
            month: 5,
            kind: MediaKind::Image,
            size: 10,
            mtime_ns: 20,
            fingerprint: fingerprint.to_string(),
        }
    }

    #[test]
    fn init_is_idempotent_and_preserves_rows() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("state.sqlite");
        {
            let mut db = StateDb::open(&path).unwrap();
            let run = db.start_run("start").unwrap();
            db.upsert_scanned(&[scanned("2024/05/a.jpg", "10:20")], run)
                .unwrap();
            db.set_thumb(
                "2024/05/a.jpg",
                Some("thumbs/2024/05/a.jpg.webp"),
                Some("a-14"),
                ThumbStatus::Ok,
                None,
            )
            .unwrap();
        }
        let db = StateDb::open(&path).unwrap();
        assert_eq!(db.item_count().unwrap(), 1);
        let items = db.load_items().unwrap();
        let item = items.get("2024/05/a.jpg").unwrap();
        assert_eq!(item.thumb_status, ThumbStatus::Ok);
        assert_eq!(item.thumb_version.as_deref(), Some("a-14"));
    }

    #[test]
    fn changed_fingerprint_resets_derived_state() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("state.sqlite");
        let mut db = StateDb::open(&path).unwrap();
        let run = db.start_run("start").unwrap();
        db.upsert_scanned(&[scanned("2024/05/a.jpg", "10:20")], run)
            .unwrap();
        db.set_probe(
            "2024/05/a.jpg",
            Some(100),
            Some(80),
            None,
            Some("2024-05-01 10:00:00"),
        )
        .unwrap();
        db.set_thumb(
            "2024/05/a.jpg",
            Some("thumbs/2024/05/a.jpg.webp"),
            Some("a-14"),
            ThumbStatus::Ok,
            None,
        )
        .unwrap();

        db.upsert_scanned(&[scanned("2024/05/a.jpg", "11:21")], run)
            .unwrap();
        let item = db.load_items().unwrap().remove("2024/05/a.jpg").unwrap();
        assert_eq!(item.thumb_status, ThumbStatus::Pending);
        assert!(item.thumb_path.is_none());
        assert!(item.width.is_none());
        assert!(item.captured_at.is_none());
    }

    #[test]
    fn deletes_unseen_rows() {
        let temp = tempfile::tempdir().unwrap();
        let mut db = StateDb::open(&temp.path().join("state.sqlite")).unwrap();
        let run1 = db.start_run("start").unwrap();
        db.upsert_scanned(
            &[
                scanned("2024/05/a.jpg", "10:20"),
                scanned("2024/05/b.jpg", "11:20"),
            ],
            run1,
        )
        .unwrap();
        let run2 = db.start_run("start").unwrap();
        db.upsert_scanned(&[scanned("2024/05/a.jpg", "10:20")], run2)
            .unwrap();
        assert_eq!(db.delete_unseen(run2).unwrap(), 1);
        assert_eq!(db.item_count().unwrap(), 1);
    }

    #[test]
    fn rejects_unknown_schema_version() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("state.sqlite");
        {
            let conn = Connection::open(&path).unwrap();
            init_schema(&conn).unwrap();
            conn.execute(
                "UPDATE meta SET value = '99' WHERE key = 'schema_version'",
                [],
            )
            .unwrap();
        }
        let error = StateDb::open(&path).unwrap_err();
        assert!(error.to_string().contains("schema version"));
    }
}
