use std::time::Duration;

use anyhow::{Context as _, Result};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use todo_list::TodoListId;

use crate::tui_app::State;

pub(crate) enum Message {
    LoadTodos,
    DecrementItem,
    IncrementItem,
    SelectTodoList(TodoListId),
    NewTodoList,
    NewItem,
    EditItem,
    DeleteItem,
    ToggleItemComplete,
    CommitTextInput,
    CancelTextInput,
    /// Insert a character at cursor position
    InsertChar(char),
    /// Delete character before cursor
    Backspace,
    /// Delete character at cursor
    Delete,
    CursorLeft,
    CursorRight,
    Quit,
}

impl Message {
    pub(crate) fn from_event(state: &State) -> Result<Option<Message>> {
        // some states automatically transition
        if let Some(message) = Self::automatic_state_transitions(state) {
            return Ok(Some(message));
        }

        // otherwise, wait a bit for a key event
        if !event::poll(Duration::from_millis(100)).context("polling for event")? {
            return Ok(None);
        }
        let Event::Key(key_event) = event::read().context("reading event")? else {
            return Ok(None);
        };

        if let Some(message) = Self::stateful_keys(state, key_event) {
            return Ok(Some(message));
        }

        Ok(matches!(key_event.code, KeyCode::Esc | KeyCode::Char('q')).then_some(Message::Quit))
    }

    /// Automatically advance the state given a previous state
    fn automatic_state_transitions(state: &State) -> Option<Message> {
        match state {
            State::Initial => Some(Message::LoadTodos),
            _ => None,
        }
    }

    /// Match key inputs according to the current state
    fn stateful_keys(state: &State, key_event: KeyEvent) -> Option<Message> {
        match state {
            State::ListSelect {
                list_state, ids, ..
            } => match key_event.code {
                KeyCode::Up => Some(Self::DecrementItem),
                KeyCode::Down => Some(Self::IncrementItem),
                KeyCode::Enter => {
                    let idx = list_state.selected()?;
                    let id = ids.get(idx)?.to_owned();
                    Some(Self::SelectTodoList(id))
                }
                KeyCode::Char('n') => Some(Self::NewTodoList),
                _ => None,
            },
            State::ListView { .. } => {
                match key_event.code {
                    // esc returns to list select
                    KeyCode::Esc => Some(Self::LoadTodos),
                    KeyCode::Up => Some(Self::DecrementItem),
                    KeyCode::Down => Some(Self::IncrementItem),
                    KeyCode::Char(' ') => Some(Self::ToggleItemComplete),
                    KeyCode::Char('n') => Some(Self::NewItem),
                    KeyCode::Char('e') => Some(Self::EditItem),
                    KeyCode::Char('x') => Some(Self::DeleteItem),
                    KeyCode::Char('q') => Some(Self::Quit),
                    _ => None,
                }
            }
            State::TextInput { .. } => {
                match key_event.code {
                    KeyCode::Esc => Some(Self::CancelTextInput),
                    KeyCode::Enter => Some(Self::CommitTextInput),
                    KeyCode::Backspace => Some(Self::Backspace),
                    KeyCode::Delete => Some(Self::Delete),
                    KeyCode::Left => Some(Self::CursorLeft),
                    KeyCode::Right => Some(Self::CursorRight),
                    KeyCode::Char(c) => {
                        // Don't insert control characters
                        if key_event.modifiers.contains(KeyModifiers::CONTROL) {
                            None
                        } else {
                            Some(Self::InsertChar(c))
                        }
                    }
                    _ => None,
                }
            }
            State::Initial | State::Error(_) | State::Exit => None,
        }
    }
}
