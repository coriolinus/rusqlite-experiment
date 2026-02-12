mod database;
mod error;

use std::collections::HashMap;

use wasm_bindgen::prelude::*;

pub use database::Database;
pub use error::{Context, Error, Result};

macro_rules! console_log {
    ($e:expr) => {{
        let js_value = JsValue::from($e);
        web_sys::console::log_1(&js_value);
    }};
    ($e:expr; $( $k:literal => $v:expr ),*) => {{
        let json = serde_json::json!({
            "msg": $e,
            "context": {
                $(
                    $k: $v
                ),*
            }
        });
        let js_value = serde_wasm_bindgen::to_value(&json).unwrap_or_else(Into::into);
        web_sys::console::table_1(&js_value);
    }};
}

macro_rules! log_call {
    ($e:expr => $f:expr) => {{
        let result = $f;
        console_log!($e; "success" => result.is_ok());
        result
    }};
}

#[wasm_bindgen]
pub async fn apply_schema(database: &mut Database) -> Result<()> {
    let mut result = todo_list::apply_schema(&mut database.connection).await;
    if let Err(err) = &result
        && err.to_string() == "applying schema"
        && let Some(err) = err.source()
        && err.to_string().ends_with("already exists")
    {
        result = Ok(());
    }
    log_call!("called apply_schema" => result.map_err(Into::into))
}

#[wasm_bindgen]
pub struct Item(todo_list::Item);

#[wasm_bindgen]
impl Item {
    pub fn id(&self) -> u32 {
        self.0.id().into()
    }

    pub fn list_id(&self) -> u32 {
        self.0.list_id().into()
    }

    pub fn description(&self) -> String {
        self.0.description().to_owned()
    }

    pub fn set_description(&mut self, description: String) {
        console_log!("called Item::set_description");
        self.0.set_description(description);
    }

    pub fn is_completed(&self) -> bool {
        self.0.is_completed()
    }

    pub fn set_is_completed(&mut self, is_completed: bool) {
        console_log!("called Item::set_is_completed");
        self.0.set_is_completed(is_completed);
    }

    /// Unix timestamp of the creation time of this item
    pub fn created_at(&self) -> u32 {
        self.0
            .created_at()
            .unix_timestamp()
            .try_into()
            .unwrap_or_default()
    }
}

#[wasm_bindgen]
pub struct TodoList(todo_list::TodoList);

#[wasm_bindgen]
impl TodoList {
    pub fn id(&self) -> u32 {
        self.0.id().into()
    }

    pub fn title(&self) -> String {
        self.0.title().to_owned()
    }

    pub fn set_title(&mut self, title: String) {
        console_log!("called TodoList::set_title");
        self.0.set_title(title);
    }

    /// Unix timestamp of hte creation time of this item
    pub fn created_at(&self) -> u32 {
        self.0
            .created_at()
            .unix_timestamp()
            .try_into()
            .unwrap_or_default()
    }

    /// Get all item ids in the database
    pub fn item_ids(&self) -> Vec<u32> {
        self.0.items().keys().copied().map(Into::into).collect()
    }

    /// Get an item by its id
    pub fn item(&self, item_id: u32) -> Option<Item> {
        self.0.item(item_id.into()).cloned().map(Item)
    }

    /// Get all todo lists with their ids
    #[wasm_bindgen(unchecked_return_type = "Record<number, string>")]
    pub async fn list_all(database: &Database) -> Result<JsValue> {
        let items = log_call!(
            "called TodoList::list_all" => todo_list::TodoList::list_all(&database.connection).await
        )?;
        let items = items
            .into_iter()
            .map(|(id, title)| (id.into(), title))
            .collect::<HashMap<u32, _>>();
        let items = serde_wasm_bindgen::to_value(&items).map_err(JsValue::from)?;
        Ok(items)
    }

    /// Create a todo list
    pub async fn new(database: &Database, title: String) -> Result<Self> {
        log_call!("called TodoList::new" => todo_list::TodoList::new(&database.connection, title).await)
            .map(Self)
            .map_err(Into::into)
    }

    /// Save a todo list and all its items
    pub async fn save(&mut self, database: &Database) -> Result<()> {
        log_call!("called TodoList::save" => self.0.save(&database.connection).await.map_err(Into::into))
    }

    /// Load a todo list by its id
    pub async fn load(database: &Database, id: u32) -> Result<Self> {
        log_call!("called TodoList::load" => todo_list::TodoList::load(&database.connection, id.into()).await)
            .map(Self)
            .map_err(Into::into)
    }

    /// Delete a todo list by its id
    ///
    /// Returns `true` if a list existed for that id.
    pub async fn delete(database: &Database, id: u32) -> Result<bool> {
        log_call!("called TodoList::delete" => todo_list::TodoList::delete(&database.connection, id.into()).await)
            .map_err(Into::into)
    }

    /// Add an item to this todo list
    ///
    /// Returns the item id.
    pub async fn add_item(&mut self, database: &Database, description: String) -> Result<u32> {
        log_call!(
            "called TodoList::add_item" => self.0.add_item(&database.connection, description).await
        )
        .map(Into::into)
        .map_err(Into::into)
    }

    /// Remove an item from this todo list.
    ///
    /// Returns `true` if an item existed for that id.
    pub async fn remove_item(&mut self, database: &Database, item_id: u32) -> Result<bool> {
        log_call!(
            "called TodoList::remove_item" => self.0.remove_item(&database.connection, item_id.into()).await
        )
        .map_err(Into::into)
    }
}
