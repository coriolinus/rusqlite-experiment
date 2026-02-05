//! Implementations to update the app state according to incoming messages.

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style, Stylize as _},
    symbols::border,
    text::{Line, Span},
    widgets::{Block, HighlightSpacing, List, ListDirection, Paragraph, Wrap},
};

use crate::tui_app::{App, State, TextInputMode};

impl App {
    /// Render the TUI according to the current state
    pub(crate) fn view(&mut self, frame: &mut Frame) {
        match &mut self.state {
            State::Initial => {
                frame.render_widget("spinning up (<q> or <esc> to quit)", frame.area())
            }
            State::ListSelect {
                labels, list_state, ..
            } => {
                let block = Self::make_block(
                    "Select a Todo list",
                    [
                        ("Navigate", "↑↓"),
                        ("Select", "enter"),
                        ("New", "n"),
                        ("Quit", "q/esc"),
                    ],
                );

                let items: Vec<Line> = if labels.is_empty() {
                    vec![Line::from(" (no lists yet - press 'n' to create one)").italic()]
                } else {
                    labels
                        .iter()
                        .map(|label| Line::from(label.as_str()))
                        .collect()
                };

                let list = List::new(items)
                    .block(block)
                    .highlight_spacing(HighlightSpacing::Always)
                    .highlight_symbol("> ")
                    .direction(ListDirection::TopToBottom);

                frame.render_stateful_widget(list, frame.area(), list_state);
            }
            State::ListView {
                todo_list,
                item_list_state,
            } => {
                let block = Self::make_block(
                    todo_list.title().as_str(),
                    [
                        ("Navigate", "↑↓"),
                        ("Toggle", "space"),
                        ("New", "n"),
                        ("Edit", "e"),
                        ("Delete", "x"),
                        ("Back", "esc"),
                        ("Quit", "q"),
                    ],
                );

                let mut items = todo_list
                    .items()
                    .values()
                    .map(|item| {
                        let description = item.description();
                        let is_completed = item.is_completed();
                        let checkbox = if is_completed { "[✓] " } else { "[ ] " };
                        let style = if is_completed {
                            Style::default().fg(Color::DarkGray)
                        } else {
                            Style::default()
                        };
                        Line::from(vec![
                            Span::raw(checkbox),
                            Span::styled(description.as_str(), style),
                        ])
                    })
                    .collect::<Vec<_>>();
                if items.is_empty() {
                    items = vec![Line::from(" (no items yet - press 'n' to create one)").italic()];
                }

                let list = List::new(items)
                    .block(block)
                    .highlight_spacing(HighlightSpacing::Always)
                    .highlight_symbol("> ")
                    .direction(ListDirection::TopToBottom);

                frame.render_stateful_widget(list, frame.area(), item_list_state);
            }
            State::TextInput {
                mode,
                buffer,
                cursor_pos,
            } => {
                // Center a modal dialog
                let modal_area = Self::centered_rect(60, 20, frame.area());

                let title = match mode {
                    TextInputMode::NewList => " Create New Todo List ",
                    TextInputMode::NewItem { .. } => " Create New Item ",
                    TextInputMode::EditItem { .. } => " Edit Item ",
                };

                let block = Self::make_block(title, [("Confirm", "enter"), ("Cancel", "esc")])
                    .border_set(border::ROUNDED);

                // Create the text display with cursor
                let text_with_cursor = if buffer.is_empty() {
                    vec![Line::from(vec![Span::styled(
                        "█",
                        Style::default().add_modifier(Modifier::REVERSED),
                    )])]
                } else {
                    let before = &buffer[..*cursor_pos];
                    let cursor_char = if *cursor_pos < buffer.len() {
                        &buffer[*cursor_pos..*cursor_pos + 1]
                    } else {
                        " "
                    };
                    let after = if *cursor_pos < buffer.len() {
                        &buffer[*cursor_pos + 1..]
                    } else {
                        ""
                    };

                    vec![Line::from(vec![
                        Span::raw(before),
                        Span::styled(
                            cursor_char,
                            Style::default().add_modifier(Modifier::REVERSED),
                        ),
                        Span::raw(after),
                    ])]
                };

                let paragraph = Paragraph::new(text_with_cursor)
                    .block(block)
                    .wrap(Wrap { trim: false });

                frame.render_widget(paragraph, modal_area);
            }
            State::Error(_) | State::Exit => {
                unreachable!("app should always exit prior to rendering this")
            }
        }
    }

    /// Helper function to create a centered rectangle
    fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
        let popup_layout = Layout::vertical([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

        Layout::horizontal([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
    }

    /// Helper function to create a text block
    fn make_block<'a>(
        title: &'a str,
        help: impl IntoIterator<Item = (&'a str, &'a str)>,
    ) -> Block<'a> {
        let title = Line::from(format!(" {title} ").bold());
        let help = Line::from(
            help.into_iter()
                .flat_map(|(action, key)| [format!(" {action} ").into(), format!("<{key}>").blue()])
                .collect::<Vec<_>>(),
        );
        Block::bordered()
            .border_set(border::PLAIN)
            .title(title)
            .title_bottom(help.centered())
    }
}
