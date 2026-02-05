use ratatui::widgets::ListState;
use todo_list::{ItemId, TodoList, TodoListId};

/// Application state
#[derive(Debug, Default)]
pub(crate) enum State {
    #[default]
    Initial,
    ListSelect {
        ids: Vec<TodoListId>,
        labels: Vec<String>,
        list_state: ListState,
    },
    ListView {
        todo_list: TodoList,
        /// Index into the items vec (derived from todo_list.items())
        item_list_state: ListState,
    },
    TextInput {
        /// What we're doing with this text input
        mode: TextInputMode,
        /// The actual text buffer
        buffer: String,
        /// Cursor position in the buffer
        cursor_pos: usize,
    },
    Error(anyhow::Error),
    Exit,
}

#[derive(Debug, Clone)]
pub(crate) enum TextInputMode {
    /// Creating a new todo list
    NewList,
    /// Creating a new item in the current list
    NewItem { list_id: TodoListId },
    /// Editing an existing item
    EditItem {
        list_id: TodoListId,
        item_id: ItemId,
    },
}

impl State {
    /// `true` when no further processing should occur if this state is reached
    pub(crate) fn is_terminal(&self) -> bool {
        matches!(self, Self::Exit | Self::Error(_))
    }
}
