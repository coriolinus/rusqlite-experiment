use std::path::Path;

use anyhow::{Context as _, Result, anyhow};
use glob::glob;
use ratatui::{
    Frame,
    style::Stylize,
    symbols::border,
    text::Line,
    widgets::{Block, HighlightSpacing, List, ListDirection, ListState},
};
use turso::Connection;

use crate::tui_app::{Message, State};

#[derive(Debug)]
pub(crate) struct App {
    pub(crate) connection: Connection,
    pub(crate) state: State,
}

impl App {
    pub(crate) async fn new(db_path: impl AsRef<Path>) -> Result<Self> {
        let db_path = std::path::absolute(db_path).context("absolutizing path")?;

        let db_exists = std::fs::exists(&db_path).context("checking for db path existence")?;

        // ensure parent path exists
        let parent = db_path
            .parent()
            .ok_or(anyhow!("cannot use `/` as the db"))?;
        std::fs::create_dir_all(parent).context("creating db parent dir")?;

        let db_path = db_path
            .to_str()
            .context("db_path could not be represented as unicode")?;
        let database = turso::Builder::new_local(db_path)
            .build()
            .await
            .context("building database")?;
        let mut connection = database.connect().context("connecting to database")?;

        if !db_exists {
            todo_list::apply_schema(&mut connection)
                .await
                .context("applying schema to new database file")
                .inspect_err(|_err| {
                    // best effort
                    // first the db itself
                    let _ = std::fs::remove_file(db_path);
                    // then ancillary files by glob if necessary
                    if let Some(paths) = glob(&format!("{db_path}*")).ok() {
                        for path in paths {
                            if let Some(path) = path.ok() {
                                let _ = std::fs::remove_file(path);
                            }
                        }
                    }
                })?;
        }

        Ok(Self {
            connection,
            state: State::Initial,
        })
    }

    /// Process an incoming message, updating the app state appropriately.
    pub(crate) async fn update(&mut self, msg: Message) -> Option<Message> {
        /// if this is an error, produce the error state and return
        macro_rules! or_err_state {
            ($e:expr) => {{
                match $e {
                    Ok(ok) => ok,
                    Err(err) => {
                        self.state = State::Error(err);
                        return None;
                    }
                }
            }};
        }

        match msg {
            Message::Quit => {
                self.state = State::Exit;
            }
            Message::LoadTodos => {
                let (ids, labels) = or_err_state!(
                    todo_list::TodoList::list_all(&self.connection)
                        .await
                        .context("listing all todo lists")
                )
                .into_iter()
                .unzip::<_, _, Vec<_>, Vec<_>>();

                self.state = State::ListSelect {
                    ids,
                    labels,
                    list_state: ListState::default(),
                };
            }
            Message::DecrementItem => match &mut self.state {
                State::ListSelect { list_state, .. } => {
                    list_state.select_previous();
                }
                state => {
                    self.state = State::Error(anyhow!(
                        "unexpected Message::DecrementItem in state: {state:?}"
                    ))
                }
            },
            Message::IncrementItem => match &mut self.state {
                State::ListSelect { list_state, .. } => {
                    list_state.select_next();
                }
                state => {
                    self.state = State::Error(anyhow!(
                        "unexpected Message::IncrementItem in state: {state:?}"
                    ))
                }
            },
            Message::SelectTodoList(list_id) => {
                self.state = State::Error(anyhow!(
                    "unimplemented: update for SelectTodoList({list_id:?})"
                ))
            }
            Message::NewTodoList => {
                self.state = State::Error(anyhow!("unimplemented: update for NewTodoList"))
            }
        }
        None
    }

    /// Render the TUI according to the current state
    pub(crate) fn view(&mut self, frame: &mut Frame) {
        match &mut self.state {
            State::Initial => {
                frame.render_widget("spinning up (<q> or <esc> to quit)", frame.area())
            }
            State::ListSelect {
                labels, list_state, ..
            } => {
                let title = Line::from(" Select a Todo list ".bold());
                let instructions = Line::from(vec![
                    " Decrement ".into(),
                    "<up>".blue(),
                    " Increment ".into(),
                    "<down>".blue(),
                    " Select ".into(),
                    "<enter>".blue(),
                    " New Todo ".into(),
                    "n".blue(),
                ]);
                let block = Block::bordered()
                    .title(title)
                    .title_bottom(instructions.centered())
                    .border_set(border::PLAIN);

                let list = List::new(labels.iter().map(|label| label.as_str()))
                    .block(block)
                    .highlight_spacing(HighlightSpacing::Always)
                    .highlight_symbol("> ")
                    .direction(ListDirection::TopToBottom);

                frame.render_stateful_widget(list, frame.area(), list_state);
            }
            State::Error(_) | State::Exit => {
                unreachable!("app should always exit prior to rendering this")
            }
        }
    }
}
