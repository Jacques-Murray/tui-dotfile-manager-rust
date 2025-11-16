// Author: Jacques Murray
//! TUI rendering logic.

use super::app::{App, AppMode};
use crate::core::diff::{DiffLineTag, DiffResult};
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
/// 2. Profile list for selection (sync mode) or backup list (restore mode)
/// 3. Log output showing sync operations and messages
pub fn render(f: &mut Frame, app: &App) {
    match app.mode {
        AppMode::Sync => render_sync_view(f, app),
        AppMode::Restore => render_restore_view(f, app),
        AppMode::DiffPreview => render_diff_preview_view(f, app),
    }
}

/// Renders the sync view with profiles.
fn render_sync_view(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(HELP_HEIGHT),
            Constraint::Min(MIN_PROFILE_HEIGHT),
            Constraint::Percentage(LOG_PERCENTAGE),
        ])
        .split(f.size());

    render_sync_help(f, app, chunks[0]);
    render_profile_list(f, app, chunks[1]);
    render_log_output(f, app, chunks[2]);
}

/// Renders the restore view with backups and preview.
fn render_restore_view(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(HELP_HEIGHT),
            Constraint::Min(MIN_PROFILE_HEIGHT),
            Constraint::Min(10),
            Constraint::Percentage(LOG_PERCENTAGE),
        ])
        .split(f.size());

    render_restore_help(f, app, chunks[0]);
    render_backup_list(f, app, chunks[1]);
    render_backup_preview(f, app, chunks[2]);
    render_log_output(f, app, chunks[3]);
}

/// Renders the help bar showing available key bindings for sync mode.
///
/// Displays different content when a sync is in progress.
fn render_sync_help(f: &mut Frame, app: &App, area: Rect) {
    let text = if let Some(progress) = &app.sync_progress {
        // Show progress bar and stats
        let percentage = if progress.total > 0 {
            (progress.current as f64 / progress.total as f64 * 100.0) as u16
        } else {
            0
        };
        Line::from(vec![
            Span::styled(
                "SYNC: ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("{}/{} ({}%) - ", progress.current, progress.total, percentage)),
            Span::styled(&progress.current_file, Style::default().fg(Color::Cyan)),
        ])
    } else if app.sync_in_progress {
        Line::from(vec![
            Span::styled(
                "SYNC IN PROGRESS...",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Please wait."),
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
            Span::styled(
                "  (R) ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Reload Config"),
            Span::styled(
                "  (r) ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Restore Mode"),
        ])
    };

    let help = Paragraph::new(text).block(Block::default().title(" Help ").borders(Borders::ALL));
    f.render_widget(help, area);
}

/// Renders the help bar showing available key bindings for restore mode.
fn render_restore_help(f: &mut Frame, app: &App, area: Rect) {
    let text = if app.restore_in_progress {
        Line::from(vec![
            Span::styled(
                "RESTORE IN PROGRESS...",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Please wait."),
        ])
    } else {
        Line::from(vec![
            Span::styled(
                "  (r) ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Restore"),
            Span::styled(
                "  (d) ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Dry Run"),
            Span::styled(
                "  (Del) ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Delete"),
            Span::styled(
                "  (b) ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Back"),
        ])
    };

    let help = Paragraph::new(text).block(
        Block::default()
            .title(" Help - Restore Mode ")
            .borders(Borders::ALL),
    );
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

/// Renders the backup list with the current selection highlighted.
fn render_backup_list(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .backups
        .iter()
        .map(|backup| {
            let display_text = format!(
                "{} - {}",
                backup.original_name,
                backup.timestamp.format("%Y-%m-%d %H:%M:%S")
            );
            ListItem::new(display_text)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().title(" Backups ").borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    // Create a ListState to manage the selection
    let mut state = ListState::default();
    state.select(app.selected_backup);

    f.render_stateful_widget(list, area, &mut state);
}

/// Renders the preview panel for the selected backup.
fn render_backup_preview(f: &mut Frame, app: &App, area: Rect) {
    let text = if let Some(selected_index) = app.selected_backup {
        if let Some(backup) = app.backups.get(selected_index) {
            let mut lines = vec![
                Line::from(format!("Original: {}", backup.original_name)),
                Line::from(format!("Target: {}", backup.target_path.display())),
                Line::from(format!(
                    "Timestamp: {}",
                    backup.timestamp.format("%Y-%m-%d %H:%M:%S")
                )),
                Line::from(format!("Size: {}", backup.format_size())),
                Line::from(""),
                Line::from("Content Preview:"),
            ];

            // Try to read first few lines of the backup
            match backup.preview_content(5) {
                Ok(preview_lines) => {
                    for line in preview_lines {
                        lines.push(Line::from(line));
                    }
                }
                Err(_) => {
                    lines.push(Line::from("[Binary file or read error]"));
                }
            }

            lines
        } else {
            vec![Line::from("No backup selected")]
        }
    } else {
        vec![Line::from("No backup selected")]
    };

    let preview = Paragraph::new(text)
        .block(Block::default().title(" Preview ").borders(Borders::ALL))
        .wrap(Wrap { trim: true });

    f.render_widget(preview, area);
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

/// Renders the diff preview view.
fn render_diff_preview_view(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(HELP_HEIGHT),
            Constraint::Min(10),
            Constraint::Percentage(30),
        ])
        .split(f.size());

    render_diff_help(f, app, chunks[0]);
    render_diff_content(f, app, chunks[1]);
    render_log_output(f, app, chunks[2]);
}

/// Renders the help bar for diff preview mode.
fn render_diff_help(f: &mut Frame, _app: &App, area: Rect) {
    let text = Line::from(vec![
        Span::styled(
            "  (q) ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("Back"),
        Span::styled(
            "  (j/k) ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("Scroll"),
        Span::styled(
            "  (n/N) ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("Next/Prev Diff"),
    ]);

    let help = Paragraph::new(text).block(
        Block::default()
            .title(" Help - Diff Preview Mode ")
            .borders(Borders::ALL),
    );
    f.render_widget(help, area);
}

/// Renders the diff content panel.
fn render_diff_content(f: &mut Frame, app: &App, area: Rect) {
    if app.diffs.is_empty() {
        let text = vec![Line::from("No diffs generated yet...")];
        let paragraph = Paragraph::new(text)
            .block(Block::default().title(" Diff Preview ").borders(Borders::ALL))
            .wrap(Wrap { trim: true });
        f.render_widget(paragraph, area);
        return;
    }

    let selected_index = app.selected_diff.unwrap_or(0);
    if selected_index >= app.diffs.len() {
        return;
    }

    let diff = &app.diffs[selected_index];
    let mut lines = Vec::new();

    // Add header with file info
    let title = format!(
        " Diff {}/{}: {} ",
        selected_index + 1,
        app.diffs.len(),
        diff.target_path().display()
    );

    // Build diff content
    match diff {
        DiffResult::NoDiff { reason, .. } => {
            lines.push(Line::from(vec![
                Span::styled("ℹ ", Style::default().fg(Color::Blue)),
                Span::raw(reason),
            ]));
        }
        DiffResult::FileDiff { diff_lines, .. } => {
            for diff_line in diff_lines {
                let line = match diff_line.tag {
                    DiffLineTag::Insert => Line::from(vec![
                        Span::styled(
                            "+ ",
                            Style::default()
                                .fg(Color::Green)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(&diff_line.content, Style::default().fg(Color::Green)),
                    ]),
                    DiffLineTag::Delete => Line::from(vec![
                        Span::styled(
                            "- ",
                            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(&diff_line.content, Style::default().fg(Color::Red)),
                    ]),
                    DiffLineTag::Equal => Line::from(vec![
                        Span::raw("  "),
                        Span::styled(&diff_line.content, Style::default().fg(Color::Gray)),
                    ]),
                };
                lines.push(line);
            }
        }
        DiffResult::NewFile {
            content_preview, ..
        } => {
            lines.push(Line::from(vec![
                Span::styled(
                    "✨ New file will be created",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
            lines.push(Line::from(""));
            lines.push(Line::from("Content preview:"));
            for (i, line) in content_preview.iter().enumerate() {
                if i >= 50 {
                    lines.push(Line::from(format!(
                        "... ({} more lines)",
                        content_preview.len() - i
                    )));
                    break;
                }
                lines.push(Line::from(vec![
                    Span::styled(
                        "+ ",
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(line, Style::default().fg(Color::Green)),
                ]));
            }
        }
        DiffResult::BinaryFile { .. } => {
            lines.push(Line::from(vec![
                Span::styled("⚠ ", Style::default().fg(Color::Yellow)),
                Span::raw("Binary file - cannot show diff"),
            ]));
        }
        DiffResult::Error { error, .. } => {
            lines.push(Line::from(vec![
                Span::styled("✗ ", Style::default().fg(Color::Red)),
                Span::styled(format!("Error: {}", error), Style::default().fg(Color::Red)),
            ]));
        }
    }

    let paragraph = Paragraph::new(lines)
        .block(Block::default().title(title).borders(Borders::ALL))
        .wrap(Wrap { trim: true })
        .scroll((app.diff_scroll, 0));

    f.render_widget(paragraph, area);
}
