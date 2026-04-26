use std::fs::OpenOptions;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

use rusqlite::OpenFlags;

use super::schema::{read_page_row, sql_err};
use super::{MediaPackReader, PageKey};

impl MediaPackReader {
    pub fn open_read(root: &Path) -> io::Result<Self> {
        let db_path = root.join("pack.sqlite");
        let bin_path = root.join("pack.bin");

        let conn = rusqlite::Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(sql_err)?;
        let _ = conn.pragma_update(None, "query_only", true);

        let file = OpenOptions::new().read(true).open(bin_path)?;

        Ok(Self { conn, file })
    }

    pub fn read_page(&mut self, key: PageKey) -> io::Result<Vec<u8>> {
        let row: Option<(u64, u64)> = read_page_row(&self.conn, key)?;

        let Some((offset, len)) = row else {
            return Err(io::Error::new(io::ErrorKind::NotFound, "pack miss"));
        };

        let len_usize = usize::try_from(len)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "page too large"))?;
        let mut buf = vec![0u8; len_usize];
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.read_exact(&mut buf)?;
        Ok(buf)
    }
}
