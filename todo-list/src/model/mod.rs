mod item;
mod todo_list;

pub use item::{Item, ItemId};
pub use todo_list::{TodoList, TodoListId};

use anyhow::{Context as _, Result};
use time::{UtcDateTime, format_description::StaticFormatDescription, macros::format_description};

static SQLITE_TIMESTAMP_FORMAT: StaticFormatDescription =
    format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");

fn parse_date(sql_date: &str) -> Result<UtcDateTime> {
    UtcDateTime::parse(sql_date, SQLITE_TIMESTAMP_FORMAT)
        .context(format!("parse_date: parsing the date ({sql_date:?})"))
}
