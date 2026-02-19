use std::path::PathBuf;

use anyhow::anyhow;
use wasm_bindgen::prelude::*;

use super::OPFS_DIRECTORY;
use crate::{Context as _, Database, Result};

const SQL_FILE: &str = "TODO: we need to figure out what this is actually called";

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

        // for debugging purposes
        for entry in opfs_project::read_dir(OPFS_DIRECTORY)
            .await
            .context("reading opfs directory")?
            .into_iter()
            .filter(|entry| {
                entry
                    .file_type()
                    .ok()
                    .is_some_and(|file_type| file_type.is_file())
            })
        {
            let log_line = format!(
                "discovered file in opfs directory: {}",
                entry.path().display()
            );
            web_sys::console::log_1(&log_line.into());
        }

        return Err(anyhow!("we do not yet know the sql file path").into());

        let path = {
            let mut path = PathBuf::from(OPFS_DIRECTORY);
            path.push(SQL_FILE);
            path
        };

        // it's too bad that we have to read the whole file given that we only need 16 bytes;
        // a real FS abstraction lets you do that kind of thing. It feels like the OPFS crate
        // ecosystem is very young still.
        let data = opfs_project::read(path)
            .await
            .context("reading sqlite file from opfs")?;

        Ok(!data.starts_with(SQLITE_MAGIC))
    }
}
