use color_eyre::eyre::{Ok, Result};
use crossterm::event::{Event, KeyCode, read};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
};
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct HyprMonitor {
    name: String,
    refresh_rate: f32,
    available_modes: Vec<String>,
}

#[derive(Debug)]
struct Monitor {
    name: String,
    refresh_rate: String,
}

struct App {
    monitors: Vec<Monitor>,
    list_state: ListState,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = run(terminal);
    ratatui::restore();
    result
}

fn get_monitors() -> Result<Vec<Monitor>> {
    let monitors = get_hypr_monitors()?;
    let monitor_names: Vec<Monitor> = monitors
        .iter()
        .flat_map(|monitor| {
            monitor.available_modes.iter().filter_map(|m| {
                m.split("@").nth(1).map(|h| Monitor {
                    refresh_rate: h.to_string(),
                    name: monitor.name.clone(),
                })
            })
        })
        .collect();
    Ok(monitor_names)
}

fn run(mut terminal: DefaultTerminal) -> Result<()> {
    let monitors = get_monitors()?;
    let mut app = App {
        monitors: monitors,
        list_state: ListState::default(),
    };

    app.list_state.select(Some(0));
    loop {
        let _ = terminal.draw(|f| render(f, &mut app));
        if let Event::Key(k) = read()? {
            match k.code {
                KeyCode::Esc => break,
                KeyCode::Up => next(&mut app),
                KeyCode::Down => previous(&mut app),
                _ => {}
            }
        }
    }
    Ok(())
}

fn next(app: &mut App) {
    let i = match app.list_state.selected() {
        Some(i) if i + 1 < app.monitors.len() => i + 1,
        _ => 0,
    };
    app.list_state.select(Some(i));
}

fn previous(app: &mut App) {
    let i = match app.list_state.selected() {
        Some(i) if i > 0 => i - 1,
        _ => app.monitors.len().saturating_sub(1),
    };
    app.list_state.select(Some(i));
}

fn render(frame: &mut Frame, app: &mut App) {
    let rect = frame.area();
    let items: Vec<ListItem> = app
        .monitors
        .iter()
        .map(|m| ListItem::new(format!("{} @ {}", m.name, m.refresh_rate)))
        .collect();
    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![
            Constraint::Min(1),
            Constraint::Percentage(70),
            Constraint::Min(1),
        ])
        .split(rect);

    let control_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![
            Constraint::Min(1),
            Constraint::Length(3),
            Constraint::Length(4),
            Constraint::Length(10),
            Constraint::Min(1),
        ])
        .split(layout[1]);

    let active_monitor = Block::default()
        .borders(Borders::ALL)
        .title("active monitor");
    let search_monitor = Block::default()
        .borders(Borders::ALL)
        .title("search monitor monitor");

    let monitor_list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("monitors list"),
        )
        .highlight_symbol(">")
        .highlight_style(Style::default().bg(Color::Blue).fg(Color::Black));

    frame.render_widget(active_monitor, control_layout[1]);
    frame.render_widget(search_monitor, control_layout[2]);
    frame.render_stateful_widget(
        monitor_list,
        control_layout[3],
        &mut app.list_state,
    );
}

fn get_hypr_monitors() -> Result<Vec<HyprMonitor>> {
    let hyprctl_output =
        Command::new("hyprctl").args(["monitors", "-j"]).output()?;
    let hyprctl_json_output_string = String::from_utf8(hyprctl_output.stdout)?;
    let hyprctl_monitors =
        serde_json::from_str::<Vec<HyprMonitor>>(&hyprctl_json_output_string)?;
    Ok(hyprctl_monitors)
}
