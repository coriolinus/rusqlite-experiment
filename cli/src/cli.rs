use std::{path::PathBuf, sync::LazyLock};

static DEFAULT_DB_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    let mut path =
        dirs::data_local_dir().expect("this will only ever run on systems with a local dir");
    path.push("todo-list");
    path.push("db.sqlite");
    path
});

#[derive(Debug, Clone, Copy, derive_more::Display, clap::ValueEnum)]
#[display(rename_all = "snake_case")]
pub(crate) enum Level {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl From<Level> for log::LevelFilter {
    fn from(value: Level) -> Self {
        match value {
            Level::Trace => log::LevelFilter::Trace,
            Level::Debug => log::LevelFilter::Debug,
            Level::Info => log::LevelFilter::Info,
            Level::Warn => log::LevelFilter::Warn,
            Level::Error => log::LevelFilter::Error,
        }
    }
}

#[derive(Debug, clap::Parser)]
pub(crate) struct Args {
    /// Path to the database
    #[arg(short='p', long, default_value = DEFAULT_DB_PATH.clone().into_os_string())]
    pub(crate) db_path: PathBuf,

    /// Enable logging
    ///
    /// If this flag is set without an explicit level argument, defaults to "info".
    #[arg(short, long, value_name = "LEVEL", num_args = 0..=1, default_missing_value = "info")]
    pub(crate) log: Option<Level>,
}
