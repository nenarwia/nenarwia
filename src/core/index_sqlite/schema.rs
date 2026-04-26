use std::path::Path;

use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};

use crate::core::scanner::FileItem;

pub(super) fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS media (
            id INTEGER PRIMARY KEY,
            rel_path TEXT NOT NULL UNIQUE,
            size INTEGER NOT NULL,
            modified_ms INTEGER NOT NULL,
            width INTEGER NOT NULL,
            height INTEGER NOT NULL,
            asset_key INTEGER NOT NULL DEFAULT 0,
            present INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_media_present ON media(present);
        CREATE INDEX IF NOT EXISTS idx_media_rel_path ON media(rel_path);",
    )?;
    Ok(())
}

pub(super) fn ensure_column(
    conn: &Connection,
    table: &str,
    column: &str,
    decl: &str,
) -> Result<()> {
    let pragma = format!("PRAGMA table_info({table})");
    let mut stmt = conn.prepare(&pragma)?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
    for row in rows {
        if row? == column {
            return Ok(());
        }
    }
    let alter = format!("ALTER TABLE {table} ADD COLUMN {column} {decl}");
    conn.execute(&alter, [])?;
    Ok(())
}

pub(super) fn next_id(conn: &Connection) -> Result<u64> {
    let val: Option<String> = conn
        .query_row("SELECT value FROM meta WHERE key = 'next_id'", [], |row| {
            row.get(0)
        })
        .optional()?;
    let parsed = val.and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
    Ok(parsed)
}

pub(super) fn set_next_id(conn: &Connection, next_id: u64) -> Result<()> {
    conn.execute(
        "INSERT INTO meta (key, value) VALUES ('next_id', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![next_id.to_string()],
    )?;
    Ok(())
}

pub(super) fn build_items(conn: &Connection, root: &Path) -> Result<Vec<FileItem>> {
    let mut stmt = conn.prepare(
        "SELECT id, rel_path, width, height, asset_key FROM media WHERE present = 1 ORDER BY id",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, u64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, u32>(2)?,
            row.get::<_, u32>(3)?,
            row.get::<_, u64>(4)?,
        ))
    })?;

    let mut out = Vec::new();
    for row in rows {
        let (id, rel, w, h, asset_key) = row?;
        out.push(FileItem {
            id,
            asset_key,
            path: root.join(rel),
            width: w,
            height: h,
        });
    }
    Ok(out)
}
