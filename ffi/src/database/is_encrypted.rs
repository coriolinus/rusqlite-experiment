use std::collections::HashMap;

use crate::{Context as _, Database, Error, Result};
use anyhow::anyhow;
use serde::Deserialize as _;
use wasm_bindgen::prelude::*;

#[derive(serde::Deserialize)]
struct IdbRelaxedDataPage {
    path: String,
    offset: u32,
    #[serde(deserialize_with = "deserialize_idb_page_data")]
    data: Vec<u8>,
}

/// Page data is stored in a weird format: an object with string keys to number values
/// It's a very JS way to store binary data I guess
fn deserialize_idb_page_data<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
    // Unfortunately because we don't know whether or not we'll actually get the data in order,
    // we have to load the whole map.
    let map = HashMap::<String, u8>::deserialize(d)?;
    // we need to sort by numeric values of the keys
    let mut pairs = map
        .into_iter()
        .filter_map(|(k, v)| {
            // if we don't have a numeric key, ignore it, we can't deal with it
            let k = k.parse::<usize>().ok()?;
            Some((k, v))
        })
        .collect::<Vec<_>>();
    pairs.sort_unstable_by_key(|(key, _value)| *key);
    Ok(pairs.into_iter().map(|(_key, value)| value).collect())
}

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

        // Open the IndexedDB database where relaxed-idb stores blocks
        let factory = idb::Factory::new()
            .map_err(Error::from)
            .context("creating IDB factory")?;

        let db = factory
            .open("relaxed-idb", None)
            .map_err(Error::from)
            .context("opening relaxed-idb database")?
            .await
            .map_err(Error::from)
            .context("awaiting IDB open")?;

        // Access the blocks object store
        let tx = db
            .transaction(&["blocks"], idb::TransactionMode::ReadOnly)
            .map_err(Error::from)
            .context("creating transaction")?;

        let store = tx
            .object_store("blocks")
            .map_err(Error::from)
            .context("getting blocks object store")?;

        // Get all keys from the object store
        let keys = store
            .get_all_keys(None, None)
            .map_err(Error::from)
            .context("getting all keys")?
            .await
            .map_err(Error::from)
            .context("awaiting getting keys")?;

        // Find the first block (offset 0) for our database
        // Keys in relaxed-idb are typically [path, offset] arrays
        let first_block_key = keys
            .into_iter()
            .find(|key| {
                let Ok((name, offset)) =
                    serde_wasm_bindgen::from_value::<(String, u32)>(key.clone())
                else {
                    return false;
                };
                name == self.name && offset == 0
            })
            .ok_or_else(|| anyhow!("no blocks found for database '{}'", self.name))?;

        // Get the first block data
        let block = store
            .get(first_block_key)
            .map_err(Error::from)
            .context("getting first block")?
            .await
            .map_err(Error::from)
            .context("awaiting first block")?
            .ok_or_else(|| anyhow!("first block data not found"))?;

        // Extract the data field from the block object
        // The block structure has a 'data' field that contains the actual bytes
        let page = serde_wasm_bindgen::from_value::<IdbRelaxedDataPage>(block)
            .map_err(Error::from)
            .context("parsing idb page")?;

        if page.path != self.name {
            return Err(anyhow!("loaded wrong page: path {} != {}", page.path, self.name).into());
        }
        if page.offset != 0 {
            return Err(anyhow!("loaded wrong page: offset {} != 0", page.offset).into());
        }

        Ok(page.data.starts_with(SQLITE_MAGIC))
    }
}
