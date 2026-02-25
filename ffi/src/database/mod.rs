mod is_encrypted;
mod set_encryption;

use std::sync::LazyLock;

use crate::{Context as _, Result};
use anyhow::anyhow;
use rusqlite::Connection;
use sqlite_wasm_rs as ffi;
use sqlite_wasm_vfs::relaxed_idb::{self, RelaxedIdbCfg, RelaxedIdbUtil};
use wasm_bindgen::prelude::*;

/// A connection to a Turso database
#[wasm_bindgen]
pub struct Database {
    pub(crate) connection: Connection,
    /// The name of the database, used as the path in IndexedDB
    name: String,
    /// VFS utils
    vfs_util: RelaxedIdbUtil,
}

#[wasm_bindgen]
impl Database {
    /// Connect to a database
    pub async fn connect(name: &str) -> Result<Self> {
        static RUSQLITE_FLAGS: LazyLock<rusqlite::OpenFlags> = LazyLock::new(|| {
            rusqlite::OpenFlags::SQLITE_OPEN_CREATE | rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
        });
        const VFS_NAME: &str = "multipleciphers-relaxed-idb";

        // install relaxed-idb persistence layer as default vfs
        // note: `RelaxedIdbCfg` sets values including the name, which gets used as the IDB database name
        let vfs_util = relaxed_idb::install::<ffi::WasmOsCallback>(&RelaxedIdbCfg::default(), true)
            .await
            .map_err(|err| anyhow!("failed to install relaxed-idb vfs: {err}"))?;

        let connection =
            rusqlite::Connection::open_with_flags_and_vfs(name, *RUSQLITE_FLAGS, VFS_NAME)
                .context("opening database connection")?;
        Ok(Self {
            connection,
            name: name.to_string(),
            vfs_util,
        })
    }

    /// Get the database's name.
    ///
    /// This is equivalent to its path in IndexedDB.
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Export the database contents as raw bytes.
    pub fn export(&self) -> Result<Vec<u8>> {
        self.vfs_util.export_db(&self.name).map_err(|err| {
            anyhow!("{err}")
                .context("exporting database from relaxed-idb")
                .into()
        })
    }
}
