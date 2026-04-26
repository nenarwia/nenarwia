use std::io::{self, Seek, SeekFrom, Write};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, OptionalExtension};

use super::super::schema::{asset_id_i64, read_page_row, row_get, sql_err};
use super::super::{MediaPack, PackKind, PageKey, PageKind};
use super::next_epoch;

impl MediaPack {
    pub fn write_page(&mut self, key: PageKey, bytes: &[u8], codec: &str) -> io::Result<()> {
        if bytes.is_empty() {
            return Ok(());
        }

        let len = bytes.len() as u64;
        let old_page = read_page_row(&self.conn, key)?;

        let offset = if let Some((old_offset, old_len)) = old_page {
            if old_len >= len {
                if old_len > len {
                    let tail_offset = old_offset.saturating_add(len);
                    let tail_len = old_len.saturating_sub(len);
                    self.insert_free_block(tail_offset, tail_len)?;
                }
                old_offset
            } else {
                let new_offset = match self.take_free_block(len)? {
                    Some(offset) => offset,
                    None => {
                        self.align_to_chunk(len)?;
                        let offset = self.write_offset;
                        self.write_offset = self.write_offset.saturating_add(len);
                        offset
                    }
                };
                self.insert_free_block(old_offset, old_len)?;
                new_offset
            }
        } else {
            match self.take_free_block(len)? {
                Some(offset) => offset,
                None => {
                    self.align_to_chunk(len)?;
                    let offset = self.write_offset;
                    self.write_offset = self.write_offset.saturating_add(len);
                    offset
                }
            }
        };

        self.file.seek(SeekFrom::Start(offset))?;
        self.file.write_all(bytes)?;

        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        self.conn
            .execute(
                "INSERT OR REPLACE INTO pages (asset_id, kind, size, mip_level, tile_x, tile_y, offset, len, codec, updated_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    asset_id_i64(key.asset_id),
                    key.kind as i32,
                    key.size as u32,
                    key.mip_level as u32,
                    key.tile_x,
                    key.tile_y,
                    offset as i64,
                    len as i64,
                    codec,
                    now_ms as i64
                ],
            )
            .map_err(sql_err)?;

        let last_used = if self.kind == PackKind::Runtime {
            next_epoch()
        } else {
            0
        };
        self.upsert_page_meta(key, len, last_used)?;
        Ok(())
    }

    pub fn touch_page(&self, key: PageKey, last_used_epoch: u64) -> io::Result<()> {
        self.conn
            .execute(
                "UPDATE pages_meta SET last_used_epoch = ?7
                 WHERE asset_id = ?1 AND kind = ?2 AND size = ?3 AND mip_level = ?4 AND tile_x = ?5 AND tile_y = ?6 AND cache_kind = ?8",
                params![
                    asset_id_i64(key.asset_id),
                    key.kind as i32,
                    key.size as u32,
                    key.mip_level as u32,
                    key.tile_x,
                    key.tile_y,
                    last_used_epoch as i64,
                    self.kind.as_i32(),
                ],
            )
            .map_err(sql_err)?;
        Ok(())
    }

    pub fn bytes_for_kind(&self, kind: PackKind) -> io::Result<u64> {
        let total: Option<i64> = self
            .conn
            .query_row(
                "SELECT SUM(byte_len) FROM pages_meta WHERE cache_kind = ?1",
                params![kind.as_i32()],
                |row| row.get(0),
            )
            .optional()
            .map_err(sql_err)?;
        Ok(total.unwrap_or(0) as u64)
    }

    pub fn evict_lru(&self, kind: PackKind, bytes_to_free: u64) -> io::Result<u64> {
        if bytes_to_free == 0 {
            return Ok(0);
        }
        let mut freed = 0u64;
        let mut stmt = self
            .conn
            .prepare(
                "SELECT asset_id, kind, size, mip_level, tile_x, tile_y, byte_len
                 FROM pages_meta
                 WHERE cache_kind = ?1
                 ORDER BY last_used_epoch ASC",
            )
            .map_err(sql_err)?;
        let mut rows = stmt.query(params![kind.as_i32()]).map_err(sql_err)?;
        while let Some(row) = rows.next().map_err(sql_err)? {
            let key = PageKey {
                asset_id: row_get::<i64>(row, 0)? as u64,
                kind: match row_get::<i64>(row, 1)? {
                    0 => PageKind::Thumb,
                    _ => PageKind::Tile,
                },
                size: row_get::<i64>(row, 2)? as u16,
                mip_level: row_get::<i64>(row, 3)? as u8,
                tile_x: row_get::<i64>(row, 4)? as u32,
                tile_y: row_get::<i64>(row, 5)? as u32,
            };
            let byte_len = row_get::<i64>(row, 6)? as u64;
            self.delete_page(key)?;
            freed = freed.saturating_add(byte_len);
            if freed >= bytes_to_free {
                break;
            }
        }
        Ok(freed)
    }

    pub fn delete_asset_pages(&self, asset_id: u64) -> io::Result<()> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT asset_id, kind, size, mip_level, tile_x, tile_y, offset, len
                 FROM pages WHERE asset_id = ?1",
            )
            .map_err(sql_err)?;
        let mut rows = stmt
            .query(params![asset_id_i64(asset_id)])
            .map_err(sql_err)?;
        while let Some(row) = rows.next().map_err(sql_err)? {
            let key = PageKey {
                asset_id: row_get::<i64>(row, 0)? as u64,
                kind: match row_get::<i64>(row, 1)? {
                    0 => PageKind::Thumb,
                    _ => PageKind::Tile,
                },
                size: row_get::<i64>(row, 2)? as u16,
                mip_level: row_get::<i64>(row, 3)? as u8,
                tile_x: row_get::<i64>(row, 4)? as u32,
                tile_y: row_get::<i64>(row, 5)? as u32,
            };
            let offset = row_get::<i64>(row, 6)? as u64;
            let len = row_get::<i64>(row, 7)? as u64;
            self.insert_free_block(offset, len)?;
            self.delete_page_meta(key)?;
        }
        self.conn
            .execute(
                "DELETE FROM pages WHERE asset_id = ?1",
                params![asset_id_i64(asset_id)],
            )
            .map_err(sql_err)?;
        Ok(())
    }

    pub fn delete_page(&self, key: PageKey) -> io::Result<()> {
        if let Some((offset, len)) = read_page_row(&self.conn, key)? {
            self.insert_free_block(offset, len)?;
        }
        self.conn
            .execute(
                "DELETE FROM pages WHERE asset_id = ?1 AND kind = ?2 AND size = ?3 AND mip_level = ?4 AND tile_x = ?5 AND tile_y = ?6",
                params![
                    asset_id_i64(key.asset_id),
                    key.kind as i32,
                    key.size as u32,
                    key.mip_level as u32,
                    key.tile_x,
                    key.tile_y,
                ],
            )
            .map_err(sql_err)?;
        self.delete_page_meta(key)?;
        Ok(())
    }

    fn upsert_page_meta(&self, key: PageKey, len: u64, last_used_epoch: u64) -> io::Result<()> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO pages_meta
                 (asset_id, kind, size, mip_level, tile_x, tile_y, byte_len, last_used_epoch, cache_kind)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    asset_id_i64(key.asset_id),
                    key.kind as i32,
                    key.size as u32,
                    key.mip_level as u32,
                    key.tile_x,
                    key.tile_y,
                    len as i64,
                    last_used_epoch as i64,
                    self.kind.as_i32(),
                ],
            )
            .map_err(sql_err)?;
        Ok(())
    }

    fn delete_page_meta(&self, key: PageKey) -> io::Result<()> {
        self.conn
            .execute(
                "DELETE FROM pages_meta WHERE asset_id = ?1 AND kind = ?2 AND size = ?3 AND mip_level = ?4 AND tile_x = ?5 AND tile_y = ?6 AND cache_kind = ?7",
                params![
                    asset_id_i64(key.asset_id),
                    key.kind as i32,
                    key.size as u32,
                    key.mip_level as u32,
                    key.tile_x,
                    key.tile_y,
                    self.kind.as_i32(),
                ],
            )
            .map_err(sql_err)?;
        Ok(())
    }
}
