use std::fs;
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::core::color;
use crate::core::index::{
    asset_key_for, index_db_path, modified_to_ms, rel_path, CachedPathMetadata,
};
use crate::core::scanner::{FileItem, Scanner};

mod schema;
use schema::{build_items, ensure_column, init_schema, next_id, set_next_id};

pub struct SqliteIndex {
    conn: Connection,
}

impl SqliteIndex {
    pub fn open(_root: &Path) -> Result<Self> {
        let path = index_db_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).context("create index directory")?;
        }
        let conn = Connection::open(path).context("open sqlite index")?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.busy_timeout(Duration::from_millis(5000))?;
        init_schema(&conn)?;
        ensure_column(&conn, "media", "asset_key", "INTEGER NOT NULL DEFAULT 0")?;
        Ok(Self { conn })
    }

    pub fn refresh(&mut self, root: &Path) -> Vec<FileItem> {
        match self.refresh_inner(root) {
            Ok(items) => items,
            Err(err) => {
                log::warn!("SQLite index refresh failed: {err:?}");
                Vec::new()
            }
        }
    }

    pub fn cached_metadata_for_key(&self, key: &str) -> Option<CachedPathMetadata> {
        self.conn
            .query_row(
                "SELECT size, modified_ms, width, height, asset_key
                 FROM media
                 WHERE rel_path = ?1",
                params![key],
                |row| {
                    Ok(CachedPathMetadata {
                        size: row.get(0)?,
                        modified_ms: row.get(1)?,
                        width: row.get(2)?,
                        height: row.get(3)?,
                        asset_key: row.get(4)?,
                    })
                },
            )
            .optional()
            .ok()
            .flatten()
    }

    pub fn cache_metadata_for_key(
        &mut self,
        key: &str,
        metadata: CachedPathMetadata,
    ) -> Result<()> {
        let tx = self.conn.transaction()?;
        let existing_id: Option<u64> = tx
            .query_row(
                "SELECT id FROM media WHERE rel_path = ?1",
                params![key],
                |row| row.get(0),
            )
            .optional()?;

        if let Some(id) = existing_id {
            tx.execute(
                "UPDATE media
                 SET size = ?2, modified_ms = ?3, width = ?4, height = ?5, asset_key = ?6
                 WHERE id = ?1",
                params![
                    id,
                    metadata.size,
                    metadata.modified_ms,
                    metadata.width,
                    metadata.height,
                    metadata.asset_key
                ],
            )?;
        } else {
            let id = next_id(&tx)?;
            tx.execute(
                "INSERT INTO media (id, rel_path, size, modified_ms, width, height, asset_key, present)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0)",
                params![
                    id,
                    key,
                    metadata.size,
                    metadata.modified_ms,
                    metadata.width,
                    metadata.height,
                    metadata.asset_key
                ],
            )?;
            set_next_id(&tx, id.saturating_add(1))?;
        }

        tx.commit()?;
        Ok(())
    }

    fn refresh_inner(&mut self, root: &Path) -> Result<Vec<FileItem>> {
        let paths = Scanner::scan_image_paths(root);

        let tx = self.conn.transaction()?;
        tx.execute("UPDATE media SET present = 0", [])?;

        let mut next_id = next_id(&tx)?;

        {
            let mut select_stmt = tx.prepare(
                "SELECT id, size, modified_ms, width, height, asset_key FROM media WHERE rel_path = ?1",
            )?;
            let mut update_stmt = tx.prepare(
                "UPDATE media SET size = ?2, modified_ms = ?3, width = ?4, height = ?5, asset_key = ?6, present = 1 WHERE rel_path = ?1",
            )?;
            let mut touch_stmt = tx.prepare("UPDATE media SET present = 1 WHERE rel_path = ?1")?;
            let mut insert_stmt = tx.prepare(
                "INSERT INTO media (id, rel_path, size, modified_ms, width, height, asset_key, present) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1)",
            )?;

            for path in paths {
                let rel = rel_path(root, &path);
                let meta = match fs::metadata(&path) {
                    Ok(m) => m,
                    Err(_) => continue,
                };
                let size = meta.len();
                let modified_ms = modified_to_ms(meta.modified());

                let existing = select_stmt
                    .query_row(params![rel], |row| {
                        Ok((
                            row.get::<_, u64>(0)?,
                            row.get::<_, u64>(1)?,
                            row.get::<_, u64>(2)?,
                            row.get::<_, u32>(3)?,
                            row.get::<_, u32>(4)?,
                            row.get::<_, u64>(5)?,
                        ))
                    })
                    .optional()?;

                let asset_key = asset_key_for(&rel, size, modified_ms);

                if let Some((_id, old_size, old_mod, old_w, old_h, old_key)) = existing {
                    let changed = old_size != size
                        || old_mod != modified_ms
                        || old_w == 0
                        || old_h == 0
                        || old_key == 0
                        || old_key != asset_key;
                    if changed {
                        let (w, h) = color::image_dimensions_any(&path).unwrap_or((0, 0));
                        update_stmt.execute(params![rel, size, modified_ms, w, h, asset_key])?;
                    } else {
                        touch_stmt.execute(params![rel])?;
                    }
                } else {
                    let (w, h) = color::image_dimensions_any(&path).unwrap_or((0, 0));
                    let id = next_id;
                    next_id = next_id.saturating_add(1);
                    insert_stmt.execute(params![id, rel, size, modified_ms, w, h, asset_key])?;
                }
            }
        }

        set_next_id(&tx, next_id)?;
        tx.commit()?;

        build_items(&self.conn, root)
    }
}
