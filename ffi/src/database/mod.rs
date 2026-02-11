use crate::{Context as _, Result};
use rusqlite::Connection;
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
        let connection = rusqlite::Connection::open(name).context("opening database connection")?;
        Ok(Self { connection })
    }
}
