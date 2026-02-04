mod item;
mod todo_list;

pub use item::{Item, ItemId};
pub use todo_list::{TodoList, TodoListId};

use anyhow::{Context as _, Result};
use time::{UtcDateTime, format_description::well_known::Iso8601};
use turso::Row;

fn parse_date(row: &Row, column: usize) -> Result<UtcDateTime> {
    let date = row
        .get::<String>(column)
        .context("parse_date: getting value from row")?;
    UtcDateTime::parse(&date, &Iso8601::DATE_TIME).context("parse_date: parsing the date")
}

fn parse_id<Id>(row: &Row, column: usize) -> Result<Id>
where
    Id: From<u32>,
{
    let id = row
        .get::<u32>(column)
        .context("parse_id: getting value from row")?;
    Ok(id.into())
}
