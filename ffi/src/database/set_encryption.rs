use crate::{Context as _, Database, Result};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl Database {
    /// Decrypt the database with the provided key.
    ///
    /// This doesn't actually change the stored data; it just allows sqlite to encrypt and decrypt data transparently
    /// on its way through this connection.
    ///
    /// The passphrase is not the actual encryption key.
    /// The encryption key is derived from the passphrase in a mechanism distinct to the cipher in use.
    ///
    /// Returns an error if the database key was incorrect.
    pub fn decrypt_database(&self, passphrase: &str) -> Result<()> {
        self.ensure_sql_cipher()
            .context("ensuring that sqlcipher encryption is used")?;
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
}
