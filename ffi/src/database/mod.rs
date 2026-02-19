mod is_encrypted;
mod set_encryption;

use crate::{Context as _, Result};
use anyhow::anyhow;
use rusqlite::Connection;
use sqlite_wasm_rs as ffi;
use sqlite_wasm_vfs::sahpool::{self, OpfsSAHPoolCfgBuilder};
use wasm_bindgen::prelude::*;

const OPFS_DIRECTORY: &str = "todo-list";

/// A connection to a Turso database
#[wasm_bindgen]
pub struct Database {
    pub(crate) connection: Connection,
    /// The name of the database, used as the path in IndexedDB
    name: String,
}

#[wasm_bindgen]
impl Database {
    /// Connect to a database
    pub async fn connect(name: &str) -> Result<Self> {
        // install OPFS persistence layer as default vfs
        // note: `OpfsSAHPoolCfg` sets values including the name, which gets used as the IDB database name
        sahpool::install::<ffi::WasmOsCallback>(
            &OpfsSAHPoolCfgBuilder::default()
                .directory(OPFS_DIRECTORY)
                .build(),
            true,
        )
        .await
        .map_err(|err| anyhow!("failed to install relaxed idb vfs: {err}"))?;

        let connection = rusqlite::Connection::open(name).context("opening database connection")?;
        Ok(Self {
            connection,
            name: name.to_string(),
        })
    }
}
