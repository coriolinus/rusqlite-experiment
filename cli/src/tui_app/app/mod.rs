mod update;
mod view;

use std::path::Path;

use anyhow::{Context as _, Result, anyhow};
use glob::glob;

use rusqlite::Connection;

use crate::tui_app::State;

#[derive(Debug)]
pub(crate) struct App {
    pub(crate) connection: Connection,
    pub(crate) state: State,
    pub(crate) logging_enabled: bool,
}

impl App {
    pub(crate) async fn new(db_path: impl AsRef<Path>, logging_enabled: bool) -> Result<Self> {
        let db_path = std::path::absolute(db_path).context("absolutizing path")?;

        let db_exists = std::fs::exists(&db_path).context("checking for db path existence")?;

        // ensure parent path exists
        let parent = db_path
            .parent()
            .ok_or(anyhow!("cannot use `/` as the db"))?;
        std::fs::create_dir_all(parent).context("creating db parent dir")?;

        let connection = rusqlite::Connection::open(&db_path).context("connecting to database")?;

        if !db_exists {
            todo_list::apply_schema(&connection)
                .await
                .context("applying schema to new database file")
                .inspect_err(|_err| {
                    // best effort
                    // first the db itself
                    let _ = std::fs::remove_file(&db_path);
                    // then ancillary files by glob if necessary
                    if let Ok(paths) = glob(&format!("{}*", db_path.to_string_lossy())) {
                        for path in paths.flatten() {
                            let _ = std::fs::remove_file(path);
                        }
                    }
                })?;
        }

        Ok(Self {
            connection,
            state: State::Initial,
            logging_enabled,
        })
    }
}
