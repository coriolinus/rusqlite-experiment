use ratatui::widgets::ListState;
use todo_list::TodoListId;

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
    Error(anyhow::Error),
    Exit,
}

impl State {
    /// `true` when no further processing should occur if this state is reached
    pub(crate) fn is_terminal(&self) -> bool {
        matches!(self, Self::Exit | Self::Error(_))
    }
}
