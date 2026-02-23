use crate::{
    state::App,
    tui::{
        key_binds::keybinds,
        panels::{left, mode, workspace},
    },
};

use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::Paragraph,
    Frame,
};

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

    let content = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(20),
            Constraint::Percentage(30),
        ])
        .split(main_layout[0]);

    keybinds(frame, main_layout[1], app);
    left::panel(frame, app, content[0]);
    mode::panel(frame, app, content[1]);
    workspace::panel(frame, app, content[2]);

    if let Some(ref err) = app.error_message {
        let error_bar = Paragraph::new(err.as_str()).style(Style::default().fg(Color::Red));
        frame.render_widget(error_bar, main_layout[2]);
    }
}
