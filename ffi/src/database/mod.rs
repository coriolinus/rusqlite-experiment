mod encryption;

use std::sync::LazyLock;

use crate::{Context as _, Result};
use anyhow::anyhow;
use rusqlite::Connection;
use sqlite_wasm_rs::WasmOsCallback;
use sqlite_wasm_vfs::relaxed_idb::{self, RelaxedIdbCfg, RelaxedIdbUtil};

use wasm_bindgen::prelude::*;

static RUSQLITE_FLAGS: LazyLock<rusqlite::OpenFlags> = LazyLock::new(|| {
    rusqlite::OpenFlags::SQLITE_OPEN_CREATE | rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
});
const VFS_NAME: &str = "multipleciphers-relaxed-idb";

/// Get the VFS utility by reinstalling the VFS
// note: `RelaxedIdbCfg` sets values including the name, which gets used as the IDB database name
async fn get_vfs_util() -> Result<RelaxedIdbUtil> {
    relaxed_idb::install::<WasmOsCallback>(&RelaxedIdbCfg::default(), false)
        .await
        .map_err(|err| anyhow!("{err}"))
        .context("failed to install relaxed-idb vfs")
}

/// A connection to a database
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
    /// Connect to an unencrypted database
    pub async fn connect(name: &str) -> Result<Self> {
        let vfs_util = get_vfs_util().await?;

        if encryption::db_file_is_encrypted(name, &vfs_util)
            .context("checking whether db file is encrypted")?
        {
            return Err(anyhow!(
                "database file is encrypted but no key was provided when connecting"
            )
            .into());
        }

        let connection =
            rusqlite::Connection::open_with_flags_and_vfs(name, *RUSQLITE_FLAGS, VFS_NAME)
                .context("opening database connection")?;
        Ok(Self {
            connection,
            name: name.to_string(),
            vfs_util,
        })
    }

    /// Connect to an encrypted database
    pub async fn connect_with_key(name: &str, passphrase: &str) -> Result<Self> {
        let vfs_util = get_vfs_util().await?;

        if !encryption::db_file_is_encrypted(name, &vfs_util)
            .context("checking whether db file is encrypted")?
        {
            return Err(anyhow!(
                "database file is not encrypted but a key was provided when connecting"
            )
            .into());
        }

        let connection =
            rusqlite::Connection::open_with_flags_and_vfs(name, *RUSQLITE_FLAGS, VFS_NAME)
                .context("opening database connection")?;

        let database = Self {
            connection,
            name: name.to_string(),
            vfs_util,
        };

        database
            .decrypt(passphrase)
            .context("decrypting database during initialization")?;

        Ok(database)
    }

    /// Get the database's name.
    ///
    /// This is equivalent to its path in IndexedDB.
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Export the database contents as raw bytes.
    pub fn export(&self) -> Result<Vec<u8>> {
        self.vfs_util
            .export_db(&self.name)
            .map_err(|err| anyhow!("{err}"))
            .context("exporting database from relaxed-idb")
    }
}
