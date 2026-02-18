use anyhow::{Context as _, Result};
use rusqlite::Connection;

const SCHEMA: &str = include_str!("schema.sql");

/// Apply the schema to the database.
///
/// Must be called once on a new database before the database can be used.
/// Must not be called repeatedly on the same database.
///
/// Really we want a proper migration format, but that's too much to build right now for this demo.
pub async fn apply_schema(connection: &Connection) -> Result<()> {
    connection.execute_batch(SCHEMA).context("applying schema")
}
