use std::fs::{self, OpenOptions};
use std::io;
use std::path::Path;

use rusqlite::Connection;

use super::super::schema::{init_schema, meta_set, meta_u64, sql_err};
use super::super::{MediaPack, PackKind};
use super::{DEFAULT_CHUNK_SIZE, MAX_CHUNK_SIZE, MIN_CHUNK_SIZE};

impl MediaPack {
    pub fn open(root: &Path, kind: PackKind) -> io::Result<Self> {
        fs::create_dir_all(root)?;

        let db_path = root.join("pack.sqlite");
        let bin_path = root.join("pack.bin");

        let conn = Connection::open(db_path).map_err(sql_err)?;
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(sql_err)?;
        conn.pragma_update(None, "synchronous", "NORMAL")
            .map_err(sql_err)?;
        init_schema(&conn)?;

        let chunk_size = meta_u64(&conn, "chunk_size")?
            .unwrap_or(DEFAULT_CHUNK_SIZE)
            .clamp(MIN_CHUNK_SIZE, MAX_CHUNK_SIZE);
        meta_set(&conn, "chunk_size", chunk_size)?;

        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(bin_path)?;
        let write_offset = file.metadata()?.len();

        Ok(Self {
            conn,
            file,
            chunk_size,
            write_offset,
            kind,
        })
    }

    pub fn begin_write_batch(&mut self) -> io::Result<()> {
        self.conn
            .execute_batch("BEGIN IMMEDIATE")
            .map_err(sql_err)?;
        Ok(())
    }

    pub fn commit_write_batch(&mut self) -> io::Result<()> {
        self.conn.execute_batch("COMMIT").map_err(sql_err)?;
        Ok(())
    }

    pub fn rollback_write_batch(&mut self) -> io::Result<()> {
        let _ = self.conn.execute_batch("ROLLBACK");
        Ok(())
    }
}
