//! Implementations to update the app state according to incoming messages.

use anyhow::{Context as _, anyhow};
use ratatui::widgets::ListState;

use crate::tui_app::{App, Message, State, TextInputMode};

impl App {
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
                State::ListView {
                    item_list_state, ..
                } => {
                    item_list_state.select_previous();
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
                State::ListView {
                    item_list_state, ..
                } => {
                    item_list_state.select_next();
                }
                state => {
                    self.state = State::Error(anyhow!(
                        "unexpected Message::IncrementItem in state: {state:?}"
                    ))
                }
            },
            Message::SelectTodoList(list_id) => {
                let todo_list = or_err_state!(
                    todo_list::TodoList::load(&self.connection, list_id)
                        .await
                        .context("loading todo list")
                );

                self.state = State::ListView {
                    todo_list,
                    item_list_state: ListState::default(),
                };
            }
            Message::NewTodoList => {
                self.state = State::TextInput {
                    mode: TextInputMode::NewList,
                    buffer: String::new(),
                    cursor_pos: 0,
                };
            }
            Message::DeleteList => {
                let State::ListSelect {
                    ids,
                    list_state,
                    ..
                } = &mut self.state
                else {
                    self.state = State::Error(anyhow!(
                        "unexpected Message::DeleteList in {:?}",
                        self.state
                    ));
                    return None;
                };

                let selected_idx = list_state.selected()?;
                let &list_id = ids.get(selected_idx)?;
                
                or_err_state!(
                    todo_list::TodoList::delete(&self.connection, list_id)
                        .await
                        .context("deleting todo list")
                );

                // Reload the list view to reflect the deletion
                return Some(Message::LoadTodos);
            }
            Message::NewItem => {
                let State::ListView { todo_list, .. } = &self.state else {
                    self.state =
                        State::Error(anyhow!("unexpected Message::NewItem in {:?}", self.state));
                    return None;
                };

                let list_id = todo_list.id();
                self.state = State::TextInput {
                    mode: TextInputMode::NewItem { list_id },
                    buffer: String::new(),
                    cursor_pos: 0,
                };
            }
            Message::EditItem => {
                let State::ListView {
                    todo_list,
                    item_list_state,
                    ..
                } = &mut self.state
                else {
                    self.state =
                        State::Error(anyhow!("unexpected Message::EditItem in {:?}", self.state));
                    return None;
                };

                let selected_idx = item_list_state.selected()?;
                let items = todo_list.items().keys().copied().collect::<Vec<_>>();
                let &item_id = items.get(selected_idx)?;
                let item = todo_list.item(item_id)?;
                let description = item.description();

                let list_id = todo_list.id();

                self.state = State::TextInput {
                    mode: TextInputMode::EditItem { list_id, item_id },
                    buffer: description.clone(),
                    cursor_pos: description.len(),
                };
            }
            Message::DeleteItem => {
                let State::ListView {
                    todo_list,
                    item_list_state,
                    ..
                } = &mut self.state
                else {
                    self.state = State::Error(anyhow!(
                        "unexpected Message::DeleteItem in {:?}",
                        self.state
                    ));
                    return None;
                };

                let selected_idx = item_list_state.selected()?;
                let items = todo_list.items().keys().copied().collect::<Vec<_>>();
                let &item_id = items.get(selected_idx)?;
                or_err_state!(
                    todo_list
                        .remove_item(&self.connection, item_id)
                        .await
                        .context("deleting item")
                );
            }
            Message::ToggleItemComplete => {
                let State::ListView {
                    todo_list,
                    item_list_state,
                } = &mut self.state
                else {
                    self.state = State::Error(anyhow!(
                        "unexpected Message::ToggleItemComplete in {:?}",
                        self.state
                    ));
                    return None;
                };

                let selected_idx = item_list_state.selected()?;
                let items = todo_list.items().keys().copied().collect::<Vec<_>>();
                let &item_id = items.get(selected_idx)?;
                let item = todo_list.item_mut(item_id)?;
                item.set_is_completed(!item.is_completed());
                or_err_state!(
                    todo_list
                        .save(&self.connection)
                        .await
                        .context("saving after toggle")
                );
            }
            Message::CommitTextInput => {
                let State::TextInput { mode, buffer, .. } = &self.state else {
                    self.state = State::Error(anyhow!(
                        "unexpected Message::CommitTextInput in {:?}",
                        self.state
                    ));
                    return None;
                };

                let buffer = buffer.trim();
                if buffer.is_empty() {
                    // Empty input, just cancel
                    return Some(Message::CancelTextInput);
                }

                match mode {
                    TextInputMode::NewList => {
                        let todo_list = or_err_state!(
                            todo_list::TodoList::new(&self.connection, buffer.to_owned())
                                .await
                                .context("creating new todo list")
                        );

                        self.state = State::ListView {
                            todo_list,
                            item_list_state: ListState::default(),
                        };
                    }
                    TextInputMode::NewItem { list_id } => {
                        let list_id = *list_id;

                        // Load the list, add item, then return to view
                        let mut todo_list = or_err_state!(
                            todo_list::TodoList::load(&self.connection, list_id)
                                .await
                                .context("loading list for new item")
                        );

                        or_err_state!(
                            todo_list
                                .add_item(&self.connection, buffer.to_string())
                                .await
                                .context("adding new item")
                        );

                        self.state = State::ListView {
                            todo_list,
                            item_list_state: ListState::default(),
                        };
                    }
                    TextInputMode::EditItem { list_id, item_id } => {
                        let list_id = *list_id;
                        let item_id = *item_id;

                        let mut todo_list = or_err_state!(
                            todo_list::TodoList::load(&self.connection, list_id)
                                .await
                                .context("loading list for edit item")
                        );

                        let item = todo_list.item_mut(item_id)?;
                        item.set_description(buffer.to_string());
                        or_err_state!(
                            todo_list
                                .save(&self.connection)
                                .await
                                .context("saving edited item")
                        );
                        self.state = State::ListView {
                            todo_list,
                            item_list_state: ListState::default(),
                        };
                    }
                }
            }
            Message::CancelTextInput => {
                let State::TextInput { mode, .. } = &self.state else {
                    self.state = State::Error(anyhow!(
                        "unexpected Message::CancelTextInput in {:?}",
                        self.state
                    ));
                    return None;
                };

                match mode {
                    TextInputMode::NewList => {
                        // Go back to list select
                        return Some(Message::LoadTodos);
                    }
                    TextInputMode::NewItem { list_id }
                    | TextInputMode::EditItem { list_id, .. } => {
                        // Go back to list view
                        return Some(Message::SelectTodoList(*list_id));
                    }
                }
            }
            Message::InsertChar(c) => {
                let State::TextInput {
                    buffer, cursor_pos, ..
                } = &mut self.state
                else {
                    self.state = State::Error(anyhow!(
                        "unexpected Message::InsertChar in {:?}",
                        self.state
                    ));
                    return None;
                };

                buffer.insert(*cursor_pos, c);
                *cursor_pos += 1;
            }
            Message::Backspace => {
                let State::TextInput {
                    buffer, cursor_pos, ..
                } = &mut self.state
                else {
                    self.state =
                        State::Error(anyhow!("unexpected Message::Backspace in {:?}", self.state));
                    return None;
                };

                if *cursor_pos > 0 {
                    *cursor_pos -= 1;
                    buffer.remove(*cursor_pos);
                }
            }
            Message::Delete => {
                let State::TextInput {
                    buffer, cursor_pos, ..
                } = &mut self.state
                else {
                    self.state =
                        State::Error(anyhow!("unexpected Message::Delete in {:?}", self.state));
                    return None;
                };

                // cursor_pos might be equal to buffer.len(), which is valid but will delete nothing
                if *cursor_pos < buffer.len() {
                    buffer.remove(*cursor_pos);
                }
            }
            Message::CursorLeft => {
                let State::TextInput { cursor_pos, .. } = &mut self.state else {
                    self.state = State::Error(anyhow!(
                        "unexpected Message::CursorLeft in {:?}",
                        self.state
                    ));
                    return None;
                };

                if *cursor_pos > 0 {
                    *cursor_pos -= 1;
                }
            }
            Message::CursorRight => {
                let State::TextInput {
                    buffer, cursor_pos, ..
                } = &mut self.state
                else {
                    self.state = State::Error(anyhow!(
                        "unexpected Message::CursorRight in {:?}",
                        self.state
                    ));
                    return None;
                };

                if *cursor_pos < buffer.len() {
                    *cursor_pos += 1;
                }
            }
        }
        None
    }
}
