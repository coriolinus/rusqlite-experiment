use futures_lite::AsyncReadExt;
use wasm_bindgen::prelude::*;

use crate::{Context as _, Database, Result};

#[wasm_bindgen]
impl Database {
    /// Check if the database is encrypted by examining the first 16 bytes.
    ///
    /// An unencrypted SQLite database starts with the magic header `b"SQLite format 3\0"`.
    /// If the first 16 bytes don't match this, the database is encrypted.
    ///
    /// Returns `Ok(true)` if encrypted, `Ok(false)` if unencrypted.
    pub async fn is_encrypted(&self) -> Result<bool> {
        // SQLite magic header for unencrypted databases
        const SQLITE_MAGIC: [u8; 16] = *b"SQLite format 3\0";

        if !tokio_fs_ext::try_exists(&self.name)
            .await
            .context("checking database file existence")?
        {
            // nonexistint database is not encrypted and is not an error
            return Ok(false);
        }

        let mut buffer = [0; 16];
        let mut file = tokio_fs_ext::File::open(&self.name)
            .await
            .context("opening database file")?;
        file.read_exact(&mut buffer)
            .await
            .context("reading first 16 bytes from opfs")?;

        Ok(buffer != SQLITE_MAGIC)
    }
}
