// Author: Jacques Murray
//! TUI rendering logic.

use super::app::App;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};

// UI Layout Constants
/// Height of the help section at the top
const HELP_HEIGHT: u16 = 3;
/// Minimum height for the profile list section
const MIN_PROFILE_HEIGHT: u16 = 5;
/// Percentage of remaining space allocated to logs
const LOG_PERCENTAGE: u16 = 70;

/// Renders the entire TUI frame.
/// 
/// The UI is divided into three sections:
/// 1. Help bar showing available commands
/// 2. Profile list for selection
/// 3. Log output showing sync operations and messages
pub fn render(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(HELP_HEIGHT),
            Constraint::Min(MIN_PROFILE_HEIGHT),
            Constraint::Percentage(LOG_PERCENTAGE),
        ])
        .split(f.size());

    render_help(f, app, chunks[0]);
    render_profile_list(f, app, chunks[1]);
    render_log_output(f, app, chunks[2]);
}

/// Renders the help bar showing available key bindings.
/// 
/// Displays different content when a sync is in progress.
fn render_help(f: &mut Frame, app: &App, area: Rect) {
    let text = if app.sync_in_progress {
        Line::from(vec![
            Span::styled(
                "SYNC IN PROGRESS...",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Please wait."),
        ])
    } else {
        Line::from(vec![
            Span::styled(
                "  (q) ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Quit"),
            Span::styled(
                "  (j/k) ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Navigate"),
            Span::styled(
                "  (s) ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Sync"),
            Span::styled(
                "  (d) ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Dry Run"),
        ])
    };

    let help = Paragraph::new(text).block(Block::default().title(" Help ").borders(Borders::ALL));
    f.render_widget(help, area);
}

/// Renders the profile list with the current selection highlighted.
fn render_profile_list(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .profiles
        .iter()
        .map(|p| ListItem::new(p.as_str()))
        .collect();

    let list = List::new(items)
        .block(Block::default().title(" Profiles ").borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    // Create a ListState to manage the selection
    let mut state = ListState::default();
    state.select(app.selected_profile);

    f.render_stateful_widget(list, area, &mut state);
}

/// Renders the log output panel with auto-scrolling.
/// 
/// The log automatically scrolls to show the most recent messages.
fn render_log_output(f: &mut Frame, app: &App, area: Rect) {
    let text: Vec<Line> = app
        .logs
        .iter()
        .map(|log| Line::from(log.as_str()))
        .collect();

    let log_paragraph = Paragraph::new(text)
        .block(Block::default().title(" Log ").borders(Borders::ALL))
        .wrap(Wrap { trim: true })
        .scroll((
            app.logs.len().saturating_sub(area.height as usize - 2) as u16,
            0,
        )); // Auto-scroll to bottom

    f.render_widget(log_paragraph, area);
}
