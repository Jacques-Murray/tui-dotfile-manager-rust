// Author: Jacques Murray
//! Main application entry point.
//! Sets up the TUI, runs the event loop, and handles cleanup.

mod core;
mod tui;

use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::DisableMouseCapture,
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    io,
    path::PathBuf,
    sync::{mpsc, Arc, RwLock},
};
use tui::{
    app::{App, WorkerMessage},
    event::{self, Event},
};

/// TUI Dotfile Manager - Manage your dotfiles with symlinks
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to configuration file
    #[arg(short, long, default_value = "config.toml")]
    config: PathBuf,

    /// Profile to sync (skips TUI if provided)
    #[arg(short, long)]
    profile: Option<String>,

    /// Perform a dry run without making changes
    #[arg(short, long)]
    dry_run: bool,

    /// List available profiles and exit
    #[arg(short, long)]
    list_profiles: bool,
}

fn main() -> Result<()> {
    // Parse CLI arguments
    let args = Args::parse();

    // 1. Setup the DotfileManager
    let manager = core::DotfileManager::new(&args.config).map_err(|e| {
        // If config loading fails, we must print to stderr before the TUI is initialized.
        eprintln!("Failed to load configuration: {}", e);
        eprintln!("Config path: {}", args.config.display());
        eprintln!("Please ensure the configuration file exists and is valid.");
        e
    })?;

    // 2. List profiles mode - print profiles and exit
    if args.list_profiles {
        println!("Available profiles:");
        for profile in manager.get_profiles() {
            println!("  - {}", profile);
        }
        return Ok(());
    }

    // 3. Headless mode - execute sync directly without TUI
    if let Some(profile) = args.profile {
        let logs = manager.execute_sync(&profile, args.dry_run)?;
        for log in logs {
            println!("{}", log);
        }
        return Ok(());
    }

    // 4. Interactive TUI mode (existing behavior)
    let manager_arc = Arc::new(RwLock::new(manager));

    // Setup channels for communication
    // event_tx/event_rx: For TUI events (keys, ticks)
    // log_tx/log_rx: For logs from the sync worker thread
    let (event_tx, event_rx) = mpsc::channel();
    let (log_tx, log_rx) = mpsc::channel::<WorkerMessage>();

    // Setup the TUI
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create the App state
    let mut app = App::new(manager_arc, args.config.clone(), log_tx);

    // Start the event listener thread
    event::run(event_tx)?;

    // Run the main event loop
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

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
