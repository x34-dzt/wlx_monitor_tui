use std::time::Duration;

use color_eyre::eyre::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::{DefaultTerminal, Frame};

use crate::config::AppConfig;
use xwlm_cfg::Compositor;
use xwlm_cfg::extract::ExtractionPlan;

enum SetupPhase {
    Extraction,
    Manual,
}

struct ExtractionResult {
    plan: ExtractionPlan,
    output_path: String,
    source_files: Vec<String>,
    monitor_count: usize,
    already_consolidated: bool,
}

struct SetupState {
    input: String,
    cursor: usize,
    compositor: Compositor,
    error: Option<String>,
    phase: SetupPhase,
    extraction: Option<ExtractionResult>,
}

impl SetupState {
    fn prev_cursor(&self) -> usize {
        self.input[..self.cursor]
            .char_indices()
            .next_back()
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    fn next_cursor(&self) -> usize {
        self.input[self.cursor..]
            .char_indices()
            .nth(1)
            .map(|(i, _)| self.cursor + i)
            .unwrap_or(self.input.len())
    }
}

fn default_config_path(compositor: Compositor) -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    match compositor {
        Compositor::Hyprland => format!("{home}/.config/hypr/monitors.conf"),
        Compositor::Sway => format!("{home}/.config/sway/monitors.conf"),
        Compositor::River => format!("{home}/.config/river/monitors.conf"),
        Compositor::Unknown => String::new(),
    }
}

fn get_outputfile_name(compositor: Compositor) -> String {
    match compositor {
        Compositor::Hyprland => "monitors.conf".to_string(),
        Compositor::Sway => "output.conf".to_string(),
        _ => "monitors.conf".to_string(),
    }
}

fn attempt_extraction(compositor: Compositor) -> Option<ExtractionResult> {
    let main_config = xwlm_cfg::main_config_path(compositor)?;
    let output_filename = get_outputfile_name(compositor);

    let plan = xwlm_cfg::extract::extract_monitors(
        &main_config,
        compositor,
        &output_filename,
    )
    .ok()?;

    if !plan.has_monitors() {
        return None;
    }

    let output_path = main_config
        .parent()?
        .join(output_filename)
        .to_string_lossy()
        .to_string();

    let source_files: Vec<String> = plan
        .modified_files
        .iter()
        .map(|(p, _)| p.to_string_lossy().to_string())
        .collect();

    let monitor_count = plan
        .output_content
        .lines()
        .filter(|l| {
            let trimmed = l.trim();
            !trimmed.is_empty() && !trimmed.starts_with('#')
        })
        .count();

    let already_consolidated = plan.source_exists
        && source_files.len() <= 1
        && source_files.first().is_some_and(|f| f == &output_path);

    Some(ExtractionResult {
        plan,
        output_path,
        source_files,
        monitor_count,
        already_consolidated,
    })
}

pub fn run(
    mut terminal: DefaultTerminal,
    compositor: Compositor,
) -> Result<Option<AppConfig>> {
    let extraction = attempt_extraction(compositor);

    let (phase, config_path) = match &extraction {
        Some(result) => (SetupPhase::Extraction, result.output_path.clone()),
        None => (SetupPhase::Manual, default_config_path(compositor)),
    };
    let cursor = config_path.len();

    let mut state = SetupState {
        input: config_path,
        cursor,
        compositor,
        error: None,
        phase,
        extraction,
    };

    loop {
        terminal.draw(|f| render(f, &state))?;

        if event::poll(Duration::from_millis(50))?
            && let Event::Key(k) = event::read()?
        {
            match (&state.phase, k.code) {
                (SetupPhase::Extraction, KeyCode::Enter) => {
                    let Some(ref result) = state.extraction else {
                        continue;
                    };
                    if !result.already_consolidated
                        && let Err(e) = result.plan.apply()
                    {
                        state.error = Some(format!("Extraction failed: {e}"));
                        state.phase = SetupPhase::Manual;
                        continue;
                    }
                    return Ok(Some(AppConfig {
                        monitor_config_path: result.output_path.clone(),
                        workspace_count: 10,
                    }));
                }
                (SetupPhase::Extraction, KeyCode::Char('m')) => {
                    state.phase = SetupPhase::Manual;
                    state.input = default_config_path(compositor);
                    state.cursor = state.input.len();
                    state.error = None;
                }
                (SetupPhase::Extraction, KeyCode::Esc) => return Ok(None),

                // --- Manual phase ---
                (SetupPhase::Manual, KeyCode::Esc) => return Ok(None),
                (SetupPhase::Manual, KeyCode::Char(c)) => {
                    state.input.insert(state.cursor, c);
                    state.cursor += c.len_utf8();
                    state.error = None;
                }
                (SetupPhase::Manual, KeyCode::Backspace) => {
                    if state.cursor > 0 {
                        let prev = state.prev_cursor();
                        state.input.remove(prev);
                        state.cursor = prev;
                    }
                    state.error = None;
                }
                (SetupPhase::Manual, KeyCode::Delete) => {
                    if state.cursor < state.input.len() {
                        state.input.remove(state.cursor);
                    }
                    state.error = None;
                }
                (SetupPhase::Manual, KeyCode::Left) => {
                    if state.cursor > 0 {
                        state.cursor = state.prev_cursor();
                    }
                }
                (SetupPhase::Manual, KeyCode::Right) => {
                    if state.cursor < state.input.len() {
                        state.cursor = state.next_cursor();
                    }
                }
                (SetupPhase::Manual, KeyCode::Home) => state.cursor = 0,
                (SetupPhase::Manual, KeyCode::End) => {
                    state.cursor = state.input.len()
                }
                (SetupPhase::Manual, KeyCode::Enter) => {
                    let path = state.input.trim();
                    if path.is_empty() {
                        state.error = Some("Path cannot be empty".to_string());
                        continue;
                    }
                    let expanded = crate::config::expand_tilde(path);
                    return Ok(Some(AppConfig {
                        monitor_config_path: expanded,
                        workspace_count: 10,
                    }));
                }
                _ => {}
            }
        }
    }
}

const LOGO: &[&str] = &[
    r"░██    ░██ ░██       ░██ ░██         ░███     ░███ ",
    r" ░██  ░██  ░██       ░██ ░██         ░████   ░████ ",
    r"  ░██░██   ░██  ░██  ░██ ░██         ░██░██ ░██░██ ",
    r"   ░███    ░██ ░████ ░██ ░██         ░██ ░████ ░██ ",
    r"  ░██░██   ░██░██ ░██░██ ░██         ░██  ░██  ░██ ",
    r" ░██  ░██  ░████   ░████ ░██         ░██       ░██ ",
    r"░██    ░██ ░███     ░███ ░██████████ ░██       ░██ ",
    r"                                                   ",
];

fn render(frame: &mut Frame, state: &SetupState) {
    match state.phase {
        SetupPhase::Extraction => render_extraction(frame, state),
        SetupPhase::Manual => render_manual(frame, state),
    }
}

fn render_logo(frame: &mut Frame, area: Rect) {
    let logo_lines: Vec<Line> = LOGO
        .iter()
        .map(|line| {
            Line::from(Span::styled(*line, Style::default().fg(Color::Cyan)))
        })
        .collect();
    frame.render_widget(Paragraph::new(logo_lines), area);
}

fn render_title(frame: &mut Frame, area: Rect) {
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            "xwlm ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("first-time setup", Style::default().fg(Color::DarkGray)),
    ]));
    frame.render_widget(title, area);
}

fn render_extraction(frame: &mut Frame, state: &SetupState) {
    let extraction = match state.extraction {
        Some(ref e) => e,
        None => return,
    };

    let file_count = extraction.source_files.len().max(1) as u16;

    let [_, center_v, _] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Max(16 + file_count),
        Constraint::Fill(1),
    ])
    .areas(frame.area());

    let [_, center, _] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Max(90),
        Constraint::Fill(1),
    ])
    .areas(center_v);

    let [
        logo_area,
        title_area,
        desc_area,
        files_area,
        output_area,
        info_area,
    ] = Layout::vertical([
        Constraint::Length(9),
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Length(file_count),
        Constraint::Length(2),
        Constraint::Length(2),
    ])
    .areas(center);

    render_logo(frame, logo_area);
    render_title(frame, title_area);

    if extraction.already_consolidated {
        let desc = Paragraph::new(Line::from(Span::styled(
            format!(
                "Detected existing {} monitor config at:",
                state.compositor.label()
            ),
            Style::default().fg(Color::White),
        )));
        frame.render_widget(desc, desc_area);

        let path_line = Line::from(Span::styled(
            format!("  {}", extraction.output_path),
            Style::default().fg(Color::Cyan),
        ));
        frame.render_widget(Paragraph::new(path_line), files_area);

        frame.render_widget(Paragraph::new(""), output_area);
    } else {
        let desc = Paragraph::new(Line::from(Span::styled(
            format!(
                "Found {} monitor config line(s) in:",
                extraction.monitor_count
            ),
            Style::default().fg(Color::White),
        )));
        frame.render_widget(desc, desc_area);

        let file_lines: Vec<Line> = extraction
            .source_files
            .iter()
            .map(|f| {
                Line::from(Span::styled(
                    format!("  {f}"),
                    Style::default().fg(Color::Cyan),
                ))
            })
            .collect();
        frame.render_widget(Paragraph::new(file_lines), files_area);

        let output = Paragraph::new(Line::from(vec![
            Span::styled(
                "Consolidate to: ",
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                &extraction.output_path,
                Style::default().fg(Color::Cyan),
            ),
        ]));
        frame.render_widget(output, output_area);
    }

    if let Some(ref err) = state.error {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!(" {err}"),
                Style::default().fg(Color::Red),
            ))),
            info_area,
        );
    } else {
        let mut hints = vec![
            Span::styled("Enter ", Style::default().fg(Color::Cyan)),
            Span::styled("confirm  ", Style::default().fg(Color::DarkGray)),
        ];
        hints.push(Span::styled("m ", Style::default().fg(Color::Cyan)));
        hints.push(Span::styled(
            "manual  ",
            Style::default().fg(Color::DarkGray),
        ));
        hints.push(Span::styled("Esc ", Style::default().fg(Color::Cyan)));
        hints.push(Span::styled("quit", Style::default().fg(Color::DarkGray)));
        frame.render_widget(Paragraph::new(Line::from(hints)), info_area);
    }
}

fn render_manual(frame: &mut Frame, state: &SetupState) {
    let [_, center_v, _] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Max(19),
        Constraint::Fill(1),
    ])
    .areas(frame.area());

    let [_, center, _] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Max(90),
        Constraint::Fill(1),
    ])
    .areas(center_v);

    let [logo_area, title_area, desc_area, input_area, info_area] =
        Layout::vertical([
            Constraint::Length(9),
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Length(2),
        ])
        .areas(center);

    render_logo(frame, logo_area);
    render_title(frame, title_area);

    let desc = Paragraph::new(Line::from(Span::styled(
        format!(
            "Enter the path to your {} config file:",
            state.compositor.label()
        ),
        Style::default().fg(Color::White),
    )));
    frame.render_widget(desc, desc_area);

    let (before, after) = state.input.split_at(state.cursor);
    let cursor_char = if after.is_empty() { " " } else { &after[..1] };
    let rest = if after.len() > 1 { &after[1..] } else { "" };

    let input_line = Line::from(vec![
        Span::styled(before, Style::default().fg(Color::White)),
        Span::styled(
            cursor_char,
            Style::default().fg(Color::Black).bg(Color::White),
        ),
        Span::styled(rest, Style::default().fg(Color::White)),
    ]);

    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Blue))
        .title(" Path ");

    frame.render_widget(
        Paragraph::new(input_line).block(input_block),
        input_area,
    );

    if let Some(ref err) = state.error {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!(" {err}"),
                Style::default().fg(Color::Red),
            ))),
            info_area,
        );
    } else {
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("Enter ", Style::default().fg(Color::Cyan)),
                Span::styled("confirm  ", Style::default().fg(Color::DarkGray)),
                Span::styled("Esc ", Style::default().fg(Color::Cyan)),
                Span::styled("quit", Style::default().fg(Color::DarkGray)),
            ])),
            info_area,
        );
    }
}
