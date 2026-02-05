use std::time::Duration;

use anyhow::{Context as _, Result};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent};
use todo_list::TodoListId;

use crate::tui_app::State;

pub(crate) enum Message {
    LoadTodos,
    DecrementItem,
    IncrementItem,
    SelectTodoList(TodoListId),
    NewTodoList,
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
        if matches!(key_event.code, KeyCode::Esc | KeyCode::Char('q')) {
            return Ok(Some(Message::Quit));
        }
        Ok(Self::stateful_keys(state, key_event))
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
            _ => None,
        }
    }
}
