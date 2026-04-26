use std::io;

use rusqlite::{params, Connection, OptionalExtension};

use super::PageKey;

pub(super) fn read_page_row(conn: &Connection, key: PageKey) -> io::Result<Option<(u64, u64)>> {
    conn.query_row(
        "SELECT offset, len FROM pages WHERE asset_id = ?1 AND kind = ?2 AND size = ?3 AND mip_level = ?4 AND tile_x = ?5 AND tile_y = ?6",
        params![
            asset_id_i64(key.asset_id),
            key.kind as i32,
            key.size as u32,
            key.mip_level as u32,
            key.tile_x,
            key.tile_y
        ],
        |row| Ok((row.get::<_, u64>(0)?, row.get::<_, u64>(1)?)),
    )
    .optional()
    .map_err(sql_err)
}

pub(super) fn init_schema(conn: &Connection) -> io::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS pages (
            asset_id INTEGER NOT NULL,
            kind INTEGER NOT NULL,
            size INTEGER NOT NULL,
            mip_level INTEGER NOT NULL,
            tile_x INTEGER NOT NULL,
            tile_y INTEGER NOT NULL,
            offset INTEGER NOT NULL,
            len INTEGER NOT NULL,
            codec TEXT NOT NULL,
            updated_ms INTEGER NOT NULL,
            PRIMARY KEY(asset_id, kind, size, mip_level, tile_x, tile_y)
        );
        CREATE TABLE IF NOT EXISTS pages_meta (
            asset_id INTEGER NOT NULL,
            kind INTEGER NOT NULL,
            size INTEGER NOT NULL,
            mip_level INTEGER NOT NULL,
            tile_x INTEGER NOT NULL,
            tile_y INTEGER NOT NULL,
            byte_len INTEGER NOT NULL,
            last_used_epoch INTEGER NOT NULL,
            cache_kind INTEGER NOT NULL,
            PRIMARY KEY(asset_id, kind, size, mip_level, tile_x, tile_y, cache_kind)
        );
        CREATE TABLE IF NOT EXISTS assets (
            asset_id INTEGER PRIMARY KEY,
            rel_path TEXT NOT NULL,
            size INTEGER NOT NULL,
            modified_ms INTEGER NOT NULL,
            width INTEGER NOT NULL,
            height INTEGER NOT NULL,
            kind TEXT NOT NULL,
            codec TEXT NOT NULL,
            tile_size INTEGER NOT NULL,
            max_mip INTEGER NOT NULL,
            updated_ms INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS free_list (
            offset INTEGER NOT NULL,
            len INTEGER NOT NULL,
            freed_ms INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_pages_offset ON pages(offset);
        CREATE INDEX IF NOT EXISTS idx_pages_meta_lru ON pages_meta(cache_kind, last_used_epoch);",
    )
    .map_err(sql_err)?;
    Ok(())
}

pub(super) fn row_get<T: rusqlite::types::FromSql>(
    row: &rusqlite::Row,
    idx: usize,
) -> io::Result<T> {
    row.get(idx).map_err(sql_err)
}

pub(super) fn meta_u64(conn: &Connection, key: &str) -> io::Result<Option<u64>> {
    let val: Option<String> = conn
        .query_row(
            "SELECT value FROM meta WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .optional()
        .map_err(sql_err)?;
    Ok(val.and_then(|v| v.parse::<u64>().ok()))
}

pub(super) fn meta_set(conn: &Connection, key: &str, value: u64) -> io::Result<()> {
    conn.execute(
        "INSERT INTO meta (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value.to_string()],
    )
    .map_err(sql_err)?;
    Ok(())
}

pub(super) fn asset_id_i64(asset_id: u64) -> i64 {
    asset_id as i64
}

pub(super) fn sql_err(err: rusqlite::Error) -> io::Error {
    io::Error::other(err)
}
