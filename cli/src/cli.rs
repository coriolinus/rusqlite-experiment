use std::{path::PathBuf, sync::LazyLock};

static DEFAULT_DB_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    let mut path =
        dirs::data_local_dir().expect("this will only ever run on systems with a local dir");
    path.push("todo-list");
    path.push("db.sqlite");
    path
});

#[derive(Debug, clap::Parser)]
pub(crate) struct Args {
    /// Path to the database
    #[arg(short='p', long, default_value = DEFAULT_DB_PATH.clone().into_os_string())]
    pub(crate) db_path: PathBuf,
}
