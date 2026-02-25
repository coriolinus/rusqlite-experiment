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
        const SQLITE_MAGIC: &[u8; 16] = b"SQLite format 3\0";

        let data = self
            .export()
            .context("exporting database to check header")?;

        if data.len() <= SQLITE_MAGIC.len() {
            // whatever this file is, it's not encrypted
            return Ok(false);
        }

        let unencrypted = data.starts_with(SQLITE_MAGIC);
        Ok(!unencrypted)
    }
}
