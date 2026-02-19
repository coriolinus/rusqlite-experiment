use crate::{Database, Result};
use wasm_bindgen::prelude::*;

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

        todo!("not relaxed idb anymore; need to re-figure-out how to do this")
    }
}
