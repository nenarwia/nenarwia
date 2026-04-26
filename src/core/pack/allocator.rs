use std::io;
use std::io::{Seek, SeekFrom, Write};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, OptionalExtension};

use super::schema::sql_err;
use super::MediaPack;

impl MediaPack {
    pub(super) fn align_to_chunk(&mut self, len: u64) -> io::Result<()> {
        if len > self.chunk_size {
            return Ok(());
        }
        let offset_in_chunk = self.write_offset % self.chunk_size;
        if offset_in_chunk == 0 {
            return Ok(());
        }
        if offset_in_chunk + len > self.chunk_size {
            let pad = self.chunk_size - offset_in_chunk;
            self.file.seek(SeekFrom::Start(self.write_offset))?;
            let zeros = vec![0u8; pad as usize];
            self.file.write_all(&zeros)?;
            self.write_offset = self.write_offset.saturating_add(pad);
        }
        Ok(())
    }

    pub(super) fn insert_free_block(&self, offset: u64, len: u64) -> io::Result<()> {
        if len == 0 {
            return Ok(());
        }
        self.conn
            .execute(
                "INSERT INTO free_list (offset, len, freed_ms) VALUES (?1, ?2, ?3)",
                params![
                    offset as i64,
                    len as i64,
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as i64,
                ],
            )
            .map_err(sql_err)?;
        Ok(())
    }

    pub(super) fn take_free_block(&self, needed_len: u64) -> io::Result<Option<u64>> {
        if needed_len == 0 {
            return Ok(None);
        }

        let row: Option<(i64, i64, i64)> = self
            .conn
            .query_row(
                "SELECT rowid, offset, len
                 FROM free_list
                 WHERE len >= ?1
                 ORDER BY len ASC, freed_ms ASC
                 LIMIT 1",
                params![needed_len as i64],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(sql_err)?;

        let Some((rowid, offset_i64, len_i64)) = row else {
            return Ok(None);
        };

        if offset_i64 < 0 || len_i64 <= 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid free_list entry",
            ));
        }

        let offset = offset_i64 as u64;
        let block_len = len_i64 as u64;
        if block_len < needed_len {
            return Ok(None);
        }

        if block_len == needed_len {
            self.conn
                .execute("DELETE FROM free_list WHERE rowid = ?1", params![rowid])
                .map_err(sql_err)?;
        } else {
            let new_offset = offset.saturating_add(needed_len);
            let new_len = block_len.saturating_sub(needed_len);
            self.conn
                .execute(
                    "UPDATE free_list SET offset = ?1, len = ?2 WHERE rowid = ?3",
                    params![new_offset as i64, new_len as i64, rowid],
                )
                .map_err(sql_err)?;
        }

        Ok(Some(offset))
    }
}
