mod cli;
mod helpers;
mod tui_app;

use anyhow::{Context as _, Result, anyhow};
use clap::Parser;

use cli::Args;

use crate::tui_app::{App, Message, State};

fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logger if requested
    if let Some(log_level) = args.log.map(Into::into) {
        tui_logger::init_logger(log_level).context("initializing logger")?;
        tui_logger::set_default_level(log_level);
    }

    let logging_enabled = args.log.is_some();
    let mut app = smol::block_on(async move { App::new(&args.db_path, logging_enabled).await })
        .context("creating app")?;

    helpers::install_panic_hook();
    let mut terminal = helpers::init_terminal().context("initializing terminal")?;

    while !app.state.is_terminal() {
        // draw the ui
        terminal
            .draw(|frame| app.view(frame))
            .map_err(|err| {
                // we can't guarantee the errors just work automaticaly as real errors for Reasons, so...
                anyhow!(err.to_string())
            })
            .context("drawing current view")?;

        // process whatever messages are necessary, including follow-ons
        let mut current_msg =
            Message::from_event(&app.state).context("getting message from current event")?;
        while let Some(msg) = current_msg {
            current_msg = smol::block_on(async { app.update(msg).await });
        }
    }

    helpers::restore_terminal().context("restoring terminal")?;

    if let State::Error(err) = app.state {
        return Err(err);
    }

    debug_assert!(
        matches!(app.state, State::Exit),
        "app encountered unexpected termination"
    );

    Ok(())
}
