use std::collections::BTreeMap;

use anyhow::{Context as _, Result};
use log::debug;
use time::UtcDateTime;
use turso::{Connection, named_params};

use crate::{Item, ItemId};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, derive_more::From, derive_more::Into,
)]
pub struct TodoListId(u32);

impl turso::params::IntoValue for TodoListId {
    fn into_value(self) -> turso::Result<turso::Value> {
        Ok(turso::Value::Integer(self.0.into()))
    }
}

impl log::kv::ToValue for TodoListId {
    fn to_value(&self) -> log::kv::Value<'_> {
        self.0.to_value()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, accessory::Accessors)]
#[access(get, defaults(all(cp)))]
pub struct TodoList {
    /// ID of this list
    id: TodoListId,
    /// List title
    #[access(get(cp = false))]
    title: String,
    /// When this list was created
    created_at: UtcDateTime,
    /// Todo list items
    #[access(get(cp = false))]
    items: BTreeMap<ItemId, Item>,
    /// Whether the list has been modified since being successfully saved
    dirty: bool,
}

// accessors
impl TodoList {
    /// Set the title
    pub fn set_title(&mut self, title: String) {
        self.dirty |= title != self.title;
        self.title = title;
    }

    /// Get an item by ID
    pub fn item(&self, item_id: ItemId) -> Option<&Item> {
        self.items.get(&item_id)
    }

    /// Get an item mutably by ID
    pub fn item_mut(&mut self, item_id: ItemId) -> Option<&mut Item> {
        self.items.get_mut(&item_id)
    }
}

// db impls
impl TodoList {
    /// Get the id and title of all todo lists
    pub async fn list_all(connection: &Connection) -> Result<Vec<(TodoListId, String)>> {
        let mut stmt = connection
            .prepare_cached("SELECT id, title FROM todo_lists")
            .await
            .context("TodoList::list_all: preparing statement")?;
        let mut rows = stmt
            .query(())
            .await
            .context("TodoList::list_all: getting rows iterator")?;

        let mut out = Vec::new();
        while let Some(row) = rows
            .next()
            .await
            .context("TodoList::list_all: fetching row")?
        {
            let id = super::parse_id(&row, 0).context("TodoList::list_all: parsing row id")?;
            let title = row.get(1).context("TodoList::list_all: getting title")?;

            out.push((id, title));
        }

        debug!("count" = out.len(); "got all todo lists");

        Ok(out)
    }

    /// Create a todo list
    pub async fn new(connection: &Connection, title: String) -> Result<Self> {
        let mut stmt = connection
            .prepare_cached("INSERT INTO todo_lists (title) VALUES (?) RETURNING id, created_at")
            .await
            .context("TodoList::new: preparing statement")?;
        let row = stmt
            .query_row((title.as_str(),))
            .await
            .context("TodoList::new: getting insertion result row")?;

        let id = super::parse_id(&row, 0).context("TodoList::new: parsing row id")?;
        let created_at = super::parse_date(&row, 1).context("TodoList::new: getting created_at")?;

        debug!(id, created_at:debug; "created a new todo list");

        Ok(Self {
            id,
            title,
            created_at,
            items: BTreeMap::new(),
            dirty: false,
        })
    }

    /// Save this list, and only this list, regardless of whether it thinks it's dirty
    async fn save_inner(&mut self, connection: &Connection) -> Result<()> {
        let mut stmt = connection
            .prepare_cached("UPDATE todo_lists SET title = :title WHERE id = :id")
            .await
            .context("TodoList::save_inner: preparing statement")?;
        let affected_rows = stmt
            .execute(named_params! {":title": self.title.as_str(), ":id": self.id})
            .await
            .context("TodoList::save_inner: executing query")?;

        debug!("list_id" = self.id; "saved todo list");
        debug_assert_eq!(
            affected_rows, 1,
            "there must always exist exactly one row in our DB for an existing TodoList"
        );

        self.dirty = false;
        Ok(())
    }

    /// Persist this list's current state and the state of all relevant items to the database.
    ///
    /// Skips updates which change nothing.
    pub async fn save(&mut self, connection: &Connection) -> Result<()> {
        for item in self.items.values_mut() {
            item.save(connection)
                .await
                .context("TodoList::save: saving an item")?;
        }
        if self.dirty {
            self.save_inner(connection)
                .await
                .context("TodoList::save: saving inner")?;
        }
        Ok(())
    }

    /// Retrieve a todo list by its id
    pub async fn load(connection: &Connection, id: TodoListId) -> Result<Self> {
        let mut stmt = connection
            .prepare_cached("SELECT title, created_at FROM todo_lists WHERE id = ?")
            .await
            .context("TodoList::load: preparing statement")?;
        let row = stmt
            .query_row((id,))
            .await
            .context("TodoList::load: querying row")?;

        let title = row.get(0).context("TodoList::load: getting title")?;
        let created_at = super::parse_date(&row, 1).context("TodoList::new: getting created_at")?;

        let items = Item::load_for_list(connection, id)
            .await
            .context("TodoList::load: loading items")?;

        debug!("list_id" = id, created_at:debug; "loaded todo list by id");

        Ok(Self {
            id,
            title,
            created_at,
            items,
            dirty: false,
        })
    }

    /// Delete a todo list by its id
    ///
    /// Returns `true` if this existed or `false` if the id had already been deleted.
    ///
    /// Automatically removes related list items due to `ON DELETE CASCADE` in the schema.
    pub async fn delete(connection: &Connection, id: TodoListId) -> Result<bool> {
        let mut stmt = connection
            .prepare_cached("DELETE FROM todo_lists WHERE id = ?")
            .await
            .context("TodoList::delete: preparing statement")?;
        let affected_rows = stmt
            .execute((id,))
            .await
            .context("TodoList::delete: deleting")?;

        debug!("list_id" = id, "was_present" = affected_rows > 0; "deleted todo list by id");

        Ok(affected_rows > 0)
    }

    /// Add an item to this list.
    pub async fn add_item(
        &mut self,
        connection: &Connection,
        description: String,
    ) -> Result<ItemId> {
        let item = Item::new(connection, self.id, description)
            .await
            .context("TodoList::add_item: creating item")?;
        let item_id = item.id();
        let ejected = self.items.insert(item_id, item);
        debug_assert!(
            ejected.is_none(),
            "inserting a new item should always produce a fresh id"
        );
        debug!(item_id, "list_id" = self.id; "added an item to a list");
        Ok(item_id)
    }

    /// Remove an item from this list.
    pub async fn remove_item(&mut self, connection: &Connection, item_id: ItemId) -> Result<bool> {
        let did_remove = Item::delete(connection, item_id)
            .await
            .context("TodoList::remove_item: deleting item")?;

        let removed = self.items.remove(&item_id);
        debug_assert_eq!(
            did_remove,
            removed.is_some(),
            "DB and memory representations should always match"
        );

        debug!(item_id, "list_id" = self.id; "removed an item from a list");
        Ok(did_remove)
    }
}
