mod database;
mod error;

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

fn elide_ok<T, E>(result: &Result<T, E>) -> String
where
    E: std::fmt::Display,
{
    match result {
        Ok(_) => "OK".into(),
        Err(err) => err.to_string(),
    }
}

/// Log a function call.
///
/// In its most complex form, offers full logging of a function including parameters and result.
/// The name must be a literal string; the params can be any expression evaluating to a debug thing.
/// `result_map` must be a closure or function which accepts the borrowed form of the result of the
/// function, and returns something with a `Debug` impl.
///
/// If called with `elide_ok` in place of `result_map`, the function must return a result.
/// `Ok(_)` are replaced with the string "Ok", and errors are printed.
///
/// If `result_map` is omitted, the entire return value is emitted in debug form.
macro_rules! log_call {
    ($name:literal ($( $param:expr ),*) => $f:expr; $result_map:expr) => {{
        let mut log_str = $name.to_string();
        log_str.push('(');
        $(
            log_str.push_str(&format!("{:?}, ", $param));
        )*
        log_str.push(')');

        let result = $f;
        log_str.push_str(&format!(" -> {:?}", $result_map(&result)));
        console_log!(log_str);
        result
    }};
    ($name:literal ($( $param:expr ),*) => $f:expr) => {
        {
            log_call!($name ($($param),*) => $f; ::std::convert::identity)
        }
    }
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
    log_call!("apply_schema"() => result.map_err(Into::into))
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

    pub fn is_completed(&self) -> bool {
        log_call!("Item::is_completed"() => self.0.is_completed())
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

impl TodoList {
    fn item_mut(&mut self, item_id: u32) -> Option<&mut todo_list::Item> {
        self.0.item_mut(item_id.into())
    }
}

#[wasm_bindgen]
impl TodoList {
    pub fn id(&self) -> u32 {
        self.0.id().into()
    }

    pub fn title(&self) -> String {
        self.0.title().to_owned()
    }

    pub fn set_title(&mut self, title: String) {
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

    /// Update an item's description.
    ///
    /// Returns `Some(dirty)` if the item was found, where `dirty` indicates whether or not item will update on the next save.
    /// Returns `None` if the item was not found.
    pub fn set_item_description(&mut self, item_id: u32, description: String) -> Option<bool> {
        let item = self.item_mut(item_id)?;
        item.set_description(description);
        Some(item.dirty())
    }

    /// Update an item's checked status.
    ///
    /// Returns `Some(dirty)` if the item was found, where `dirty` indicates whether or not the item will update on the next save.
    /// Returns `None` if the item was not found.
    pub fn set_item_completed(&mut self, item_id: u32, is_completed: bool) -> Option<bool> {
        let item = self.item_mut(item_id)?;
        item.set_is_completed(is_completed);
        Some(item.dirty())
    }

    /// Get all todo lists with their ids
    #[wasm_bindgen(unchecked_return_type = "[number, string][]")]
    pub async fn list_all(database: &Database) -> Result<JsValue> {
        let items = log_call!(
            "TodoList::list_all"() =>
            todo_list::TodoList::list_all(&database.connection).await;
            elide_ok
        )?
        .into_iter()
        .map(|(id, name)| (u32::from(id), name))
        .collect::<Vec<_>>();
        let items = serde_wasm_bindgen::to_value(&items).map_err(JsValue::from)?;
        Ok(items)
    }

    /// Create a todo list
    pub async fn new(database: &Database, title: String) -> Result<Self> {
        log_call!(
            "TodoList::new"(title) =>
            todo_list::TodoList::new(&database.connection, title).await;
            elide_ok
        )
        .map(Self)
        .map_err(Into::into)
    }

    /// Save a todo list and all its items
    pub async fn save(&mut self, database: &Database) -> Result<()> {
        log_call!("TodoList::save"() => self.0.save(&database.connection).await.map_err(Into::into))
    }

    /// Load a todo list by its id
    pub async fn load(database: &Database, id: u32) -> Result<Self> {
        log_call!(
            "TodoList::load"() =>
            todo_list::TodoList::load(&database.connection, id.into()).await;
            elide_ok
        )
        .map(Self)
        .map_err(Into::into)
    }

    /// Delete a todo list by its id
    ///
    /// Returns `true` if a list existed for that id.
    pub async fn delete(database: &Database, id: u32) -> Result<bool> {
        log_call!("TodoList::delete"(id) => todo_list::TodoList::delete(&database.connection, id.into()).await)
            .map_err(Into::into)
    }

    /// Add an item to this todo list
    ///
    /// Returns the item id.
    pub async fn add_item(&mut self, database: &Database, description: String) -> Result<u32> {
        log_call!(
            "TodoList::add_item"(description) =>
            self.0.add_item(&database.connection, description).await
        )
        .map(Into::into)
        .map_err(Into::into)
    }

    /// Remove an item from this todo list.
    ///
    /// Returns `true` if an item existed for that id.
    pub async fn remove_item(&mut self, database: &Database, item_id: u32) -> Result<bool> {
        log_call!(
            "TodoList::remove_item"(item_id) =>
            self.0.remove_item(&database.connection, item_id.into()).await
        )
        .map_err(Into::into)
    }
}
