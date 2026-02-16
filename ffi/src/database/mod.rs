use crate::{Context as _, Result};
use anyhow::anyhow;
use rusqlite::Connection;
use sqlite_wasm_rs as ffi;
use sqlite_wasm_vfs::relaxed_idb::{self, RelaxedIdbCfg};
use wasm_bindgen::prelude::*;

/// A connection to a Turso database
#[wasm_bindgen]
pub struct Database {
    pub(crate) connection: Connection,
}

#[wasm_bindgen]
impl Database {
    /// Connect to a database
    pub async fn connect(name: &str) -> Result<Self> {
        // install relaxed-idb persistence layer as default vfs
        // note: `RelaxedIdbCfg` sets values including the name, which gets used as the IDB database name
        relaxed_idb::install::<ffi::WasmOsCallback>(&RelaxedIdbCfg::default(), true)
            .await
            .map_err(|err| anyhow!("failed to install relaxed idb vfs: {err}"))?;

        let connection = rusqlite::Connection::open(name).context("opening database connection")?;
        Ok(Self { connection })
    }
}
