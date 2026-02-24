mod is_encrypted;
mod set_encryption;

use crate::{Context as _, Result};
use anyhow::anyhow;
use rusqlite::Connection;
use sqlite_wasm_rs as ffi;
use sqlite_wasm_vfs::sahpool::{self, OpfsSAHPoolCfgBuilder};
use wasm_bindgen::prelude::*;

const SAHPOOL_MAGIC_DIRECTORY: &str = ".sahpool-magic";

/// A connection to a Turso database
#[wasm_bindgen]
pub struct Database {
    pub(crate) connection: Connection,
    /// The name of the database, equivalent to the path in OPFS
    name: String,
    /// VFS utils
    vfs_util: sahpool::OpfsSAHPoolUtil,
}

#[wasm_bindgen]
impl Database {
    /// Connect to a database
    pub async fn connect(name: &str) -> Result<Self> {
        // ensure sahpool magic directory exists before attempting to write it
        tokio_fs_ext::create_dir_all(SAHPOOL_MAGIC_DIRECTORY)
            .await
            .context("creating sahpool magic directory")?;

        // install OPFS persistence layer as default vfs
        // note: `OpfsSAHPoolCfg` sets values including the name, which gets used as the IDB database name
        let vfs_util = sahpool::install::<ffi::WasmOsCallback>(
            &OpfsSAHPoolCfgBuilder::default()
                .directory(SAHPOOL_MAGIC_DIRECTORY)
                .build(),
            true,
        )
        .await
        .map_err(|err| anyhow!("failed to install vfs: {err}"))?;

        let connection = rusqlite::Connection::open_with_flags_and_vfs(
            name,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE | rusqlite::OpenFlags::SQLITE_OPEN_CREATE,
            "multipleciphers-opfs-sahpool",
        )
        .context("opening database connection")?;
        Ok(Self {
            connection,
            name: name.to_string(),
            vfs_util,
        })
    }

    /// Get the database's name.
    ///
    /// This is equivalent to its path in OPFS.
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Download the database
    pub fn export(&self) -> Result<Vec<u8>> {
        self.vfs_util.export_db(&self.name).map_err(|err| {
            anyhow!("{err}")
                .context("exporting database from sahpool")
                .into()
        })
    }
}
