use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

use rusqlite::{params, Connection, OpenFlags};

use super::schema::{init_schema, row_get, sql_err};

type PageMetaKey = (i64, i64, i64, i64, i64, i64);
type PageMetaValue = (i64, i64, i64);

pub fn compact_pack(root: &Path) -> io::Result<()> {
    let db_path = root.join("pack.sqlite");
    let bin_path = root.join("pack.bin");
    if !db_path.exists() || !bin_path.exists() {
        return Ok(());
    }

    let tmp_db = root.join("pack_new.sqlite");
    let tmp_bin = root.join("pack_new.bin");
    remove_if_exists(&tmp_db)?;
    remove_if_exists(&tmp_bin)?;

    let src_conn = Connection::open_with_flags(db_path.clone(), OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(sql_err)?;
    let mut src_file = File::open(&bin_path)?;

    let dst_conn = Connection::open(&tmp_db).map_err(sql_err)?;
    init_schema(&dst_conn)?;

    {
        let mut stmt = src_conn
            .prepare("SELECT key, value FROM meta")
            .map_err(sql_err)?;
        let mut rows = stmt.query([]).map_err(sql_err)?;
        while let Some(row) = rows.next().map_err(sql_err)? {
            let key: String = row_get(row, 0)?;
            let value: String = row_get(row, 1)?;
            dst_conn
                .execute(
                    "INSERT INTO meta (key, value) VALUES (?1, ?2)",
                    params![key, value],
                )
                .map_err(sql_err)?;
        }
    }

    {
        let mut stmt = src_conn
            .prepare(
                "SELECT asset_id, rel_path, size, modified_ms, width, height, kind, codec, tile_size, max_mip, updated_ms
                 FROM assets",
            )
            .map_err(sql_err)?;
        let mut rows = stmt.query([]).map_err(sql_err)?;
        while let Some(row) = rows.next().map_err(sql_err)? {
            dst_conn
                .execute(
                    "INSERT INTO assets (asset_id, rel_path, size, modified_ms, width, height, kind, codec, tile_size, max_mip, updated_ms)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                    params![
                        row_get::<i64>(row, 0)?,
                        row_get::<String>(row, 1)?,
                        row_get::<i64>(row, 2)?,
                        row_get::<i64>(row, 3)?,
                        row_get::<i64>(row, 4)?,
                        row_get::<i64>(row, 5)?,
                        row_get::<String>(row, 6)?,
                        row_get::<String>(row, 7)?,
                        row_get::<i64>(row, 8)?,
                        row_get::<i64>(row, 9)?,
                        row_get::<i64>(row, 10)?,
                    ],
                )
                .map_err(sql_err)?;
        }
    }

    let mut meta_map: HashMap<PageMetaKey, PageMetaValue> = HashMap::new();
    {
        let mut stmt = src_conn
            .prepare(
                "SELECT asset_id, kind, size, mip_level, tile_x, tile_y, cache_kind, byte_len, last_used_epoch
                 FROM pages_meta",
            )
            .map_err(sql_err)?;
        let mut rows = stmt.query([]).map_err(sql_err)?;
        while let Some(row) = rows.next().map_err(sql_err)? {
            let key = (
                row_get::<i64>(row, 0)?,
                row_get::<i64>(row, 1)?,
                row_get::<i64>(row, 2)?,
                row_get::<i64>(row, 3)?,
                row_get::<i64>(row, 4)?,
                row_get::<i64>(row, 5)?,
            );
            let cache_kind = row_get::<i64>(row, 6)?;
            let byte_len = row_get::<i64>(row, 7)?;
            let last_used = row_get::<i64>(row, 8)?;
            meta_map.insert(key, (cache_kind, byte_len, last_used));
        }
    }

    let mut dst_file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .read(true)
        .write(true)
        .open(&tmp_bin)?;
    let mut write_offset = 0u64;

    {
        let mut stmt = src_conn
            .prepare(
                "SELECT asset_id, kind, size, mip_level, tile_x, tile_y, offset, len, codec, updated_ms
                 FROM pages
                 ORDER BY offset",
            )
            .map_err(sql_err)?;
        let mut rows = stmt.query([]).map_err(sql_err)?;
        while let Some(row) = rows.next().map_err(sql_err)? {
            let asset_id = row_get::<i64>(row, 0)?;
            let kind = row_get::<i64>(row, 1)?;
            let size = row_get::<i64>(row, 2)?;
            let mip_level = row_get::<i64>(row, 3)?;
            let tile_x = row_get::<i64>(row, 4)?;
            let tile_y = row_get::<i64>(row, 5)?;
            let offset = row_get::<i64>(row, 6)? as u64;
            let len = row_get::<i64>(row, 7)? as u64;
            let codec = row_get::<String>(row, 8)?;
            let updated_ms = row_get::<i64>(row, 9)?;

            let len_usize = usize::try_from(len)
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "page too large"))?;
            let mut buf = vec![0u8; len_usize];
            src_file.seek(SeekFrom::Start(offset))?;
            src_file.read_exact(&mut buf)?;

            dst_file.seek(SeekFrom::Start(write_offset))?;
            dst_file.write_all(&buf)?;

            dst_conn
                .execute(
                    "INSERT OR REPLACE INTO pages (asset_id, kind, size, mip_level, tile_x, tile_y, offset, len, codec, updated_ms)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    params![
                        asset_id,
                        kind,
                        size,
                        mip_level,
                        tile_x,
                        tile_y,
                        write_offset as i64,
                        len as i64,
                        codec,
                        updated_ms,
                    ],
                )
                .map_err(sql_err)?;

            let (cache_kind, byte_len, last_used) = meta_map
                .get(&(asset_id, kind, size, mip_level, tile_x, tile_y))
                .copied()
                .unwrap_or((0, len as i64, 0));

            dst_conn
                .execute(
                    "INSERT OR REPLACE INTO pages_meta
                     (asset_id, kind, size, mip_level, tile_x, tile_y, byte_len, last_used_epoch, cache_kind)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        asset_id,
                        kind,
                        size,
                        mip_level,
                        tile_x,
                        tile_y,
                        byte_len,
                        last_used,
                        cache_kind,
                    ],
                )
                .map_err(sql_err)?;

            write_offset = write_offset.saturating_add(len);
        }
    }

    drop(src_conn);
    drop(dst_conn);
    drop(src_file);
    drop(dst_file);

    replace_file(&tmp_bin, &bin_path)?;
    replace_file(&tmp_db, &db_path)?;
    Ok(())
}

fn replace_file(tmp: &Path, final_path: &Path) -> io::Result<()> {
    if !tmp.exists() {
        return Ok(());
    }
    let backup = final_path.with_extension("bak");
    if final_path.exists() {
        fs::rename(final_path, &backup)?;
    }
    if let Err(err) = fs::rename(tmp, final_path) {
        if backup.exists() {
            let _ = fs::rename(&backup, final_path);
        }
        return Err(err);
    }
    if backup.exists() {
        let _ = fs::remove_file(&backup);
    }
    Ok(())
}

fn remove_if_exists(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    }
}
