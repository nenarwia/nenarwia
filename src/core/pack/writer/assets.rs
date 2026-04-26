use std::io;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, OptionalExtension};

use super::super::schema::{asset_id_i64, sql_err};
use super::super::{AssetRecord, MediaPack};

impl MediaPack {
    pub fn asset_record(&self, asset_id: u64) -> io::Result<Option<AssetRecord>> {
        let row = self
            .conn
            .query_row(
                "SELECT rel_path, size, modified_ms, width, height, kind, codec, tile_size, max_mip
                 FROM assets WHERE asset_id = ?1",
                params![asset_id_i64(asset_id)],
                |row| {
                    Ok(AssetRecord {
                        asset_id,
                        rel_path: row.get::<_, String>(0)?,
                        size: row.get::<_, i64>(1)? as u64,
                        modified_ms: row.get::<_, i64>(2)? as u64,
                        width: row.get::<_, u32>(3)?,
                        height: row.get::<_, u32>(4)?,
                        kind: row.get::<_, String>(5)?,
                        codec: row.get::<_, String>(6)?,
                        tile_size: row.get::<_, u32>(7)?,
                        max_mip: row.get::<_, u32>(8)? as u8,
                    })
                },
            )
            .optional()
            .map_err(sql_err)?;
        Ok(row)
    }

    pub fn upsert_asset(&self, record: &AssetRecord) -> io::Result<()> {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        self.conn
            .execute(
                "INSERT INTO assets (asset_id, rel_path, size, modified_ms, width, height, kind, codec, tile_size, max_mip, updated_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                 ON CONFLICT(asset_id) DO UPDATE SET
                    rel_path = excluded.rel_path,
                    size = excluded.size,
                    modified_ms = excluded.modified_ms,
                    width = excluded.width,
                    height = excluded.height,
                    kind = excluded.kind,
                    codec = excluded.codec,
                    tile_size = excluded.tile_size,
                    max_mip = excluded.max_mip,
                    updated_ms = excluded.updated_ms",
                params![
                    asset_id_i64(record.asset_id),
                    record.rel_path.as_str(),
                    record.size as i64,
                    record.modified_ms as i64,
                    record.width as i64,
                    record.height as i64,
                    record.kind.as_str(),
                    record.codec.as_str(),
                    record.tile_size as i64,
                    record.max_mip as i64,
                    now_ms as i64,
                ],
            )
            .map_err(sql_err)?;
        Ok(())
    }
}
