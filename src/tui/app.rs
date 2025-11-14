// Author: Jacques Murray
//! Defines the TUI application state (App struct).

use crate::core::DotfileManager;
use crossterm::event::KeyCode;
use std::sync::{mpsc, Arc};
use std::thread;

/// Represents the TUI's state and logic.
pub struct App {
    pub manager: Arc<DotfileManager>,
    pub profiles: Vec<String>,
    pub selected_profile: Option<usize>,
    pub logs: Vec<String>,
    pub should_quit: bool,
    pub sync_in_progress: bool,
    log_tx: mpsc::Sender<String>,
}

impl App {
    /// Creates a new App.
    pub fn new(manager: Arc<DotfileManager>, log_tx: mpsc::Sender<String>) -> Self {
        let profiles = manager.get_profiles();
        let selected_profile = if profiles.is_empty() { None } else { Some(0) };

        Self {
            manager,
            profiles,
            selected_profile,
            logs: vec![
                "Welcome to the TUI Dotfile Manager!".to_string(),
                "Use 'j'/'k' or Arrow Up/Down to select a profile.".to_string(),
                "'s' = Sync, 'd' = Dry Run, 'q' = Quit.".to_string(),
            ],
            should_quit: false,
            sync_in_progress: false,
            log_tx,
        }
    }

    /// Handles a key press event.
    pub fn on_key(&mut self, key: KeyCode) {
        if self.sync_in_progress {
            return;
        }

        match key {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('j') | KeyCode::Down => self.select_next(),
            KeyCode::Char('k') | KeyCode::Up => self.select_previous(),
            KeyCode::Char('d') => self.start_sync(true),
            KeyCode::Char('s') | KeyCode::Enter => self.start_sync(false),
            _ => {}
        }
    }

    /// Selects the next profile in the list.
    fn select_next(&mut self) {
        if self.profiles.is_empty() {
            return;
        }
        let i = self.selected_profile.unwrap_or(0);
        let next = if i >= self.profiles.len() - 1 {
            0
        } else {
            i + 1
        };
        self.selected_profile = Some(next);
    }

    /// Selects the previous profile in the list.
    fn select_previous(&mut self) {
        if self.profiles.is_empty() {
            return;
        }
        let i = self.selected_profile.unwrap_or(0);
        let prev = if i == 0 {
            self.profiles.len() - 1
        } else {
            i - 1
        };
        self.selected_profile = Some(prev);
    }

    /// Spawns a worker thread to perform the sync.
    fn start_sync(&mut self, dry_run: bool) {
        if self.sync_in_progress {
            return;
        }

        if let Some(selected_index) = self.selected_profile {
            self.sync_in_progress = true;
            let profile_name = self.profiles[selected_index].clone();
            let manager = Arc::clone(&self.manager);
            let log_tx = self.log_tx.clone();

            self.logs.push("---".to_string());
            self.logs.push(format!(
                "Starting {}...",
                if dry_run { "Dry Run" } else { "Sync" }
            ));

            // Spawn the blocking I/O in a separate thread
            thread::spawn(move || {
                match manager.execute_sync(&profile_name, dry_run) {
                    Ok(logs) => {
                        for log in logs {
                            log_tx.send(log).ok();
                        }
                    }
                    Err(e) => {
                        log_tx.send(format!("[FATAL ERROR] {}", e)).ok();
                    }
                }
                // Send a signal that the thread is done
                log_tx.send("---SYNC_COMPLETE---".to_string()).ok();
            });
        } else {
            self.logs.push("[ERROR] No profile selected.".to_string());
        }
    }

    /// Called when the app receives a new log message.
    pub fn on_log(&mut self, log: String) {
        if log == "---SYNC_COMPLETE---" {
            self.sync_in_progress = false;
        } else {
            self.logs.push(log);
        }
    }
}
