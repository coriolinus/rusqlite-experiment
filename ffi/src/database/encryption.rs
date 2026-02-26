use crate::{Context as _, Database, Result};
use anyhow::anyhow;
use sqlite_wasm_vfs::relaxed_idb::RelaxedIdbUtil;
use wasm_bindgen::prelude::*;

// SQLite magic header for unencrypted databases
const SQLITE_MAGIC: &[u8; 16] = b"SQLite format 3\0";

/// Check whether a database file is encrypted, without requiring an open connection to that database
pub(super) fn db_file_is_encrypted(db_name: &str, vfs_util: &RelaxedIdbUtil) -> Result<bool> {
    let data = vfs_util
        .export_db(db_name)
        .map_err(|err| anyhow!("{err}"))
        .context("exporting database to check header")?;

    if data.len() <= SQLITE_MAGIC.len() {
        // whatever this file is, it's not encrypted
        return Ok(false);
    }

    let unencrypted = data.starts_with(SQLITE_MAGIC);
    Ok(!unencrypted)
}

/// Check whether a particular database is encrypted by its filename, without requiring an open connection to that database
#[wasm_bindgen]
pub async fn db_is_encrypted(db_name: &str) -> Result<bool> {
    let vfs_util = super::get_vfs_util().await?;
    db_file_is_encrypted(db_name, &vfs_util)
}

impl Database {
    /// Ensure that the cipher in use is `sqlcipher`
    ///
    /// <https://utelle.github.io/SQLite3MultipleCiphers/docs/ciphers/cipher_sqlcipher/>
    fn ensure_sql_cipher(&self) -> Result<()> {
        const CIPHER: &str = "cipher";
        const SQLCIPHER: &str = "sqlcipher";
        let existing_cipher = self
            .connection
            .pragma_query_value(None, CIPHER, |row| row.get::<_, String>(0))
            .context("getting existing cipher pragma")?;
        if existing_cipher != SQLCIPHER {
            self.connection
                .pragma_update(None, CIPHER, SQLCIPHER)
                .context("updating cipher pragma")?;
        }
        Ok(())
    }

    /// Decrypt the database with the provided key.
    ///
    /// **IMPORTANT** This must be the first operation performed on a newly opened connection.
    ///
    /// This doesn't actually change the stored data; it just allows sqlite to encrypt and decrypt data transparently
    /// on its way through this connection.
    ///
    /// The passphrase is not the actual encryption key.
    /// The encryption key is derived from the passphrase in a mechanism distinct to the cipher in use.
    ///
    /// Returns an error if the database key was incorrect.
    pub(super) fn decrypt(&self, passphrase: &str) -> Result<()> {
        self.connection
            .pragma_update(None, "key", passphrase)
            .context("setting pragma key")?;
        // the pragma itself gives no indication of whether or not the encryption key was correct.
        // its documentation suggests this as a simple fast query which can determine if decryption works.
        self.connection
            .execute("SELECT * FROM sqlite_master", [])
            .context("executing sample query failed; check the encryption key")?;
        Ok(())
    }
}

#[wasm_bindgen]
impl Database {
    /// Set the encryption key for the database.
    ///
    /// This updates the stored data such that it is all encrypted with the key derived from teh provided passphrase.
    ///
    /// The passphrase is not the actual encryption key.
    /// The encryption key is derived from the passphrase in a mechanism distinct to the cipher in use.
    ///
    /// This operation has three use cases:
    ///
    ///   1. Encrypt an existing unencrypted database
    ///   2. Change the encryption key of an existing encrypted database.
    ///   3. Remove encryption from an existing encrypted database.
    ///
    /// Removing encryption is accomplished by providing an empty passphrase.
    pub fn set_key(&self, passphrase: &str) -> Result<()> {
        self.ensure_sql_cipher()
            .context("ensuring that sqlcipher encryption is used")?;
        self.connection
            .pragma_update(None, "rekey", passphrase)
            .context("rekeying database")?;
        Ok(())
    }

    /// Check if the database is encrypted by examining the first 16 bytes.
    pub fn is_encrypted(&self) -> Result<bool> {
        db_file_is_encrypted(&self.name, &self.vfs_util)
    }
}
