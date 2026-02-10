use std::collections::BTreeMap;

use anyhow::{Context as _, Result};
use log::debug;
use time::UtcDateTime;
use turso::{Connection, named_params};

use crate::TodoListId;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, derive_more::From, derive_more::Into,
)]
pub struct ItemId(u32);

impl turso::params::IntoValue for ItemId {
    fn into_value(self) -> turso::Result<turso::Value> {
        Ok(turso::Value::Integer(self.0.into()))
    }
}

impl log::kv::ToValue for ItemId {
    fn to_value(&self) -> log::kv::Value<'_> {
        self.0.to_value()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, accessory::Accessors)]
#[access(get, defaults(all(cp)))]
pub struct Item {
    id: ItemId,
    list_id: TodoListId,
    #[access(get(cp = false))]
    description: String,
    is_completed: bool,
    created_at: UtcDateTime,
    dirty: bool,
}

// accessors
impl Item {
    /// Set the description
    pub fn set_description(&mut self, description: String) {
        self.dirty |= description != self.description;
        self.description = description;
    }

    /// Set the completion status
    pub fn set_is_completed(&mut self, is_completed: bool) {
        self.dirty |= is_completed != self.is_completed;
        self.is_completed = is_completed;
    }
}

// db impls
impl Item {
    /// Create a new Item and insert it into the DB
    pub(crate) async fn new(
        connection: &Connection,
        list_id: TodoListId,
        description: String,
    ) -> Result<Self> {
        let mut stmt = connection
            .prepare_cached(
                "INSERT INTO todo_items (list_id, description)
                VALUES (:list_id, :description)
                RETURNING id, created_at",
            )
            .await
            .context("Item::new: preparing statement")?;
        let row = stmt
            .query_row(named_params! {":list_id": list_id, ":description": description.as_str()})
            .await
            .context("Item::new: inserting row")?;

        let id = super::parse_id(&row, 0).context("Item::new: parsing inserted id")?;
        let created_at = super::parse_date(&row, 1).context("TodoList::new: getting created_at")?;

        debug!(id, list_id, created_at:debug; "inserted new Item into the db");

        Ok(Self {
            id,
            list_id,
            description,
            created_at,
            is_completed: false,
            dirty: false,
        })
    }

    /// Update this item in the DB, but only if it's dirty.
    pub(crate) async fn save(&mut self, connection: &Connection) -> Result<()> {
        if !self.dirty {
            debug!("id" = self.id; "returning early from saving Item in db because it is not dirty");
            return Ok(());
        }

        let mut stmt = connection
            .prepare_cached(
                "UPDATE todo_items
                SET description = :description, is_completed = :is_completed
                WHERE id = :id",
            )
            .await
            .context("Item::save: prepare statement")?;
        let affected_rows = stmt
            .execute(named_params! {
                ":description": self.description.as_str(),
                ":is_completed": self.is_completed,
                ":id": self.id,
            })
            .await
            .context("Item::save: execute query")?;

        debug!("id" = self.id, "is_completed" = self.is_completed; "saved Item in the db");
        debug_assert_eq!(affected_rows, 1, "each item should affect exactly one row");

        self.dirty = false;
        Ok(())
    }

    /// Load an Item by its id
    pub async fn load(connection: &Connection, id: ItemId) -> Result<Self> {
        let mut stmt = connection
            .prepare_cached(
                "SELECT list_id, description, is_completed, created_at
                FROM todo_items WHERE id = ?",
            )
            .await
            .context("Item::load: preparing statement")?;
        let row = stmt
            .query_row((id,))
            .await
            .context("Item::load: loading row")?;

        let list_id = super::parse_id(&row, 0).context("Item::load: getting list_id")?;
        let description = row.get(1).context("Item::load: getting description")?;
        let is_completed = row.get(2).context("Item::load: getting is_completed")?;
        let created_at = super::parse_date(&row, 3).context("Item::load: getting created_at")?;

        debug!(id, list_id, created_at:debug, is_completed; "loaded an item by its id");

        Ok(Self {
            id,
            list_id,
            description,
            is_completed,
            created_at,
            dirty: false,
        })
    }

    /// Load all items by todo list id
    ///
    /// Not for public use; end-users should use the `TodoList` interface instead.
    /// But this implementation supports that one.
    pub(crate) async fn load_for_list(
        connection: &Connection,
        list_id: TodoListId,
    ) -> Result<BTreeMap<ItemId, Self>> {
        let mut stmt = connection
            .prepare_cached(
                "SELECT id, description, is_completed, created_at
                FROM todo_items WHERE list_id = ?",
            )
            .await
            .context("Item::load_for_list: preparing statement")?;
        let mut rows = stmt
            .query((list_id,))
            .await
            .context("Item::load_for_list: querying rows")?;

        let mut out = BTreeMap::new();
        while let Some(row) = rows
            .next()
            .await
            .context("Item::load_for_list: getting next row")?
        {
            let id = super::parse_id(&row, 0).context("Item::load_for_list: getting id")?;
            let description = row.get(1).context("Item::load: getting description")?;
            let is_completed = row.get(2).context("Item::load: getting is_completed")?;
            let created_at =
                super::parse_date(&row, 3).context("Item::load: getting created_at")?;

            let ejected = out.insert(
                id,
                Self {
                    id,
                    list_id,
                    description,
                    is_completed,
                    created_at,
                    dirty: false,
                },
            );
            debug_assert_eq!(ejected, None);
        }

        debug!("count" = out.len(), list_id; "loaded all items by list id");

        Ok(out)
    }

    /// Delete an item by its id
    ///
    /// Not for public use; end-users shold use the `TodoList` interface instead.
    /// But this implementation supports that one.
    ///
    /// Returns true if deleting removed an actual item.
    pub(crate) async fn delete(connection: &Connection, id: ItemId) -> Result<bool> {
        let mut stmt = connection
            .prepare_cached("DELETE FROM todo_items WHERE id = ?")
            .await
            .context("Item::delete: preparing statement")?;
        let affected_rows = stmt
            .execute((id,))
            .await
            .context("Item::delete: executing delete")?;

        debug!(id, "was_present" = affected_rows > 0; "deleted an item by its id");

        Ok(affected_rows > 0)
    }
}
