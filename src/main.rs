// Author: Jacques Murray
//! Main application entry point.
//! Sets up the TUI, runs the event loop, and handles cleanup.

mod core;
mod tui;

use anyhow::Result;
use crossterm::{
    event::DisableMouseCapture,
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    io,
    path::PathBuf,
    sync::{mpsc, Arc},
};
use tui::{
    app::{App, WorkerMessage},
    event::{self, Event},
};

fn main() -> Result<()> {
    // 1. Setup the DotfileManager
    // We look for config.toml in the current directory.
    let config_path = PathBuf::from("config.toml");
    let manager = core::DotfileManager::new(&config_path).map_err(|e| {
        // If TUI setup fails, we must print to stderr.
        eprintln!("Failed to load configuration: {}", e);
        eprintln!("Please ensure 'config.toml' exists and is valid.");
        e
    })?;
    let manager_arc = Arc::new(manager);

    // 2. Setup channels for communication
    // event_tx/event_rx: For TUI events (keys, ticks)
    // log_tx/log_rx: For logs from the sync worker thread
    let (event_tx, event_rx) = mpsc::channel();
    let (log_tx, log_rx) = mpsc::channel::<WorkerMessage>();

    // 3. Setup the TUI
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, crossterm::event::EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 4. Create the App state
    let mut app = App::new(manager_arc, log_tx);

    // 5. Start the event listener thread
    event::run(event_tx)?;

    // 6. Run the main event loop
    while !app.should_quit {
        // Draw the UI
        terminal.draw(|f| tui::render(f, &app))?;

        // Check for new logs from the sync thread (non-blocking)
        if let Ok(msg) = log_rx.try_recv() {
            app.on_log(msg);
        }

        // Wait for the next TUI event (blocking)
        match event_rx.recv()? {
            Event::Key(key) => app.on_key(key),
            Event::Tick => { /* We could add tick-based logic here */ }
            Event::Error(err) => {
                // Log the error and continue - the event thread has exited
                app.on_log(WorkerMessage::Log(format!("[ERROR] Event thread: {}", err)));
            }
        }
    }

    // 7. Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
