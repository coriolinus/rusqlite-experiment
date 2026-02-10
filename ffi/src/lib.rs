mod database;
mod error;

use wasm_bindgen::prelude::*;

pub use database::Database;
pub use error::{Context, Error, Result};

#[wasm_bindgen]
pub async fn apply_schema(database: &mut Database) -> Result<()> {
    todo_list::apply_schema(&mut database.connection)
        .await
        .map_err(Into::into)
}
