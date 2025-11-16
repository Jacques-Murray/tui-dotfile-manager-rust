// Author: Jacques Murray
//! Defines the TUI application state (App struct).

use crate::core::diff::DiffResult;
use crate::core::restore::BackupEntry;
use crate::core::DotfileManager;
use crossterm::event::KeyCode;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{mpsc, Arc, RwLock};
use std::thread;

/// Maximum number of log messages to keep in memory.
/// Older messages are removed to prevent unbounded memory growth.
const MAX_LOG_MESSAGES: usize = 1000;

/// Application operating modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    /// Sync mode - select and sync profiles
    Sync,
    /// Restore mode - browse and restore backups
    Restore,
    /// Diff preview mode - view diffs before syncing
    DiffPreview,
}

/// Messages sent from worker threads to the main UI thread.
pub enum WorkerMessage {
    Log(String),
    SyncComplete,
    RestoreComplete,
    BackupsListed(Vec<BackupEntry>),
    DiffGenerated(Vec<DiffResult>),
}

/// Represents the TUI's state and logic.
///
/// The App struct maintains the application state including the selected profile,
/// log messages, and sync status. It handles user input and coordinates with
/// background worker threads for I/O operations.
pub struct App {
    pub manager: Arc<RwLock<DotfileManager>>,
    pub config_path: PathBuf,
    pub profiles: Vec<String>,
    pub selected_profile: Option<usize>,
    pub logs: VecDeque<String>,
    pub should_quit: bool,
    pub sync_in_progress: bool,
    pub sync_progress: Option<SyncProgress>,
    pub mode: AppMode,
    pub backups: Vec<BackupEntry>,
    pub selected_backup: Option<usize>,
    pub restore_in_progress: bool,
    pub diffs: Vec<DiffResult>,
    pub selected_diff: Option<usize>,
    pub diff_scroll: u16,
    log_tx: mpsc::Sender<WorkerMessage>,
}

impl App {
    /// Creates a new App.
    ///
    /// # Arguments
    /// * `manager` - Shared reference to the DotfileManager
    /// * `config_path` - Path to the configuration file for reloading
    /// * `log_tx` - Channel sender for receiving log messages from worker threads
    ///
    /// # Returns
    /// A new App instance with initial welcome messages and the first profile selected.
    pub fn new(
        manager: Arc<RwLock<DotfileManager>>,
        config_path: PathBuf,
        log_tx: mpsc::Sender<WorkerMessage>,
    ) -> Self {
        let profiles = manager.read().unwrap().get_profiles();
        let selected_profile = if profiles.is_empty() { None } else { Some(0) };

        Self {
            manager,
            config_path,
            profiles,
            selected_profile,
            logs: VecDeque::from([
                "Welcome to the TUI Dotfile Manager!".to_string(),
                "Use 'j'/'k' or Arrow Up/Down to select a profile.".to_string(),
                "'s' = Sync, 'd' = Dry Run, 'p' = Diff Preview, 'r' = Restore Mode, 'q' = Quit.".to_string(),
            ]),
            should_quit: false,
            sync_in_progress: false,
            sync_progress: None,
            mode: AppMode::Sync,
            backups: Vec::new(),
            selected_backup: None,
            restore_in_progress: false,
            diffs: Vec::new(),
            selected_diff: None,
            diff_scroll: 0,
            log_tx,
        }
    }

    /// Handles a key press event.
    ///
    /// # Key Bindings (Sync Mode)
    /// * `q` or `Esc` - Quit the application
    /// * `j` or `Down` - Select next profile
    /// * `k` or `Up` - Select previous profile
    /// * `s` or `Enter` - Start sync for selected profile
    /// * `d` - Start dry run for selected profile
    /// * `p` - Preview diff for selected profile
    /// * `r` - Enter restore mode
    ///
    /// # Key Bindings (Restore Mode)
    /// * `Esc` or `b` - Back to sync mode
    /// * `j` or `Down` - Select next backup
    /// * `k` or `Up` - Select previous backup
    /// * `r` or `Enter` - Restore selected backup
    /// * `d` - Dry run restore for selected backup
    /// * `Delete` - Delete selected backup
    ///
    /// # Key Bindings (Diff Preview Mode)
    /// * `Esc` or `q` - Back to sync mode
    /// * `j` or `Down` - Scroll down
    /// * `k` or `Up` - Scroll up
    /// * `n` - Next diff
    /// * `N` - Previous diff
    ///
    /// Input is ignored while a sync or restore operation is in progress.
    pub fn on_key(&mut self, key: KeyCode) {
        if self.sync_in_progress || self.restore_in_progress {
            return;
        }

        match self.mode {
            AppMode::Sync => self.handle_sync_mode_key(key),
            AppMode::Restore => self.handle_restore_mode_key(key),
            AppMode::DiffPreview => self.handle_diff_mode_key(key),
        }
    }

    /// Handles key presses in sync mode.
    fn handle_sync_mode_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('j') | KeyCode::Down => self.select_next_profile(),
            KeyCode::Char('k') | KeyCode::Up => self.select_previous_profile(),
            KeyCode::Char('d') => self.start_sync(true),
            KeyCode::Char('s') | KeyCode::Enter => self.start_sync(false),
            KeyCode::Char('p') => self.enter_diff_preview_mode(),
            KeyCode::Char('r') => self.enter_restore_mode(),
            _ => {}
        }
    }

    /// Handles key presses in restore mode.
    fn handle_restore_mode_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc | KeyCode::Char('b') => self.exit_restore_mode(),
            KeyCode::Char('j') | KeyCode::Down => self.select_next_backup(),
            KeyCode::Char('k') | KeyCode::Up => self.select_previous_backup(),
            KeyCode::Char('r') | KeyCode::Enter => self.start_restore(false),
            KeyCode::Char('d') => self.start_restore(true),
            KeyCode::Delete => self.delete_selected_backup(),
            _ => {}
        }
    }

    /// Selects the next profile in the list.
    ///
    /// Wraps around to the first profile when reaching the end.
    fn select_next_profile(&mut self) {
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
    ///
    /// Wraps around to the last profile when at the beginning.
    fn select_previous_profile(&mut self) {
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

    /// Selects the next backup in the list.
    ///
    /// Wraps around to the first backup when reaching the end.
    fn select_next_backup(&mut self) {
        if self.backups.is_empty() {
            return;
        }
        let i = self.selected_backup.unwrap_or(0);
        let next = if i >= self.backups.len() - 1 {
            0
        } else {
            i + 1
        };
        self.selected_backup = Some(next);
    }

    /// Selects the previous backup in the list.
    ///
    /// Wraps around to the last backup when at the beginning.
    fn select_previous_backup(&mut self) {
        if self.backups.is_empty() {
            return;
        }
        let i = self.selected_backup.unwrap_or(0);
        let prev = if i == 0 {
            self.backups.len() - 1
        } else {
            i - 1
        };
        self.selected_backup = Some(prev);
    }

    /// Enters restore mode and loads the list of backups.
    fn enter_restore_mode(&mut self) {
        self.mode = AppMode::Restore;
        self.logs.push_back("---".to_string());
        self.logs.push_back("Entering Restore Mode...".to_string());

        let manager = Arc::clone(&self.manager);
        let log_tx = self.log_tx.clone();

        // Spawn a thread to list backups
        thread::spawn(move || {
            let result = {
                let manager_read = manager.read().unwrap();
                manager_read.list_backups()
            };
            match result {
                Ok(backups) => {
                    log_tx.send(WorkerMessage::BackupsListed(backups)).ok();
                }
                Err(e) => {
                    log_tx
                        .send(WorkerMessage::Log(format!(
                            "[ERROR] Failed to list backups: {}",
                            e
                        )))
                        .ok();
                    log_tx.send(WorkerMessage::BackupsListed(Vec::new())).ok();
                }
            }
        });
    }

    /// Exits restore mode and returns to sync mode.
    fn exit_restore_mode(&mut self) {
        self.mode = AppMode::Sync;
        self.backups.clear();
        self.selected_backup = None;
        self.logs.push_back("---".to_string());
        self.logs.push_back("Exited Restore Mode".to_string());
    }

    /// Reloads the configuration file and updates the profile list.
    ///
    /// This method:
    /// - Re-reads and parses the config file
    /// - Updates the profile list
    /// - Adjusts profile selection if needed
    /// - Provides user feedback
    /// - Handles errors gracefully without crashing
    fn reload_config(&mut self) {
        if self.sync_in_progress {
            self.logs.push_back("---".to_string());
            self.logs
                .push_back("[WARN] Cannot reload config during sync.".to_string());
            return;
        }

        if self.restore_in_progress {
            self.logs.push_back("---".to_string());
            self.logs
                .push_back("[WARN] Cannot reload config during restore.".to_string());
            return;
        }

        self.logs.push_back("---".to_string());
        self.logs
            .push_back("Reloading configuration...".to_string());

        // Attempt to reload the configuration
        let result = {
            let mut manager = self.manager.write().unwrap();
            manager.reload_config(&self.config_path)
        };

        match result {
            Ok(()) => {
                // Update profile list
                let old_profiles = std::mem::take(&mut self.profiles);
                self.profiles = self.manager.read().unwrap().get_profiles();

                // Handle selection adjustment
                let old_selected_name = self
                    .selected_profile
                    .and_then(|idx| old_profiles.get(idx))
                    .cloned();

                if let Some(old_name) = old_selected_name {
                    // Try to find the previously selected profile in the new list
                    self.selected_profile = self
                        .profiles
                        .iter()
                        .position(|name| name == &old_name)
                        .or_else(|| {
                            if !self.profiles.is_empty() {
                                self.logs.push_back(format!(
                                    "[INFO] Profile '{}' no longer exists. Switched to '{}'.",
                                    old_name, self.profiles[0]
                                ));
                                Some(0)
                            } else {
                                None
                            }
                        });
                } else if !self.profiles.is_empty() {
                    self.selected_profile = Some(0);
                } else {
                    self.selected_profile = None;
                }

                // Success message
                self.logs.push_back(format!(
                    "[SUCCESS] Configuration reloaded. {} profile(s) available.",
                    self.profiles.len()
                ));
            }
            Err(e) => {
                // Error handling - don't crash, just log the error
                self.logs
                    .push_back(format!("[ERROR] Failed to reload configuration: {}", e));
                self.logs
                    .push_back("[INFO] Using previous configuration.".to_string());
            }
        }
    }

    /// Spawns a worker thread to perform the sync.
    ///
    /// # Arguments
    /// * `dry_run` - If true, performs a dry run without making changes
    ///
    /// The sync operation runs in a background thread to keep the UI responsive.
    /// Log messages are sent back via the log channel.
    fn start_sync(&mut self, dry_run: bool) {
        if self.sync_in_progress {
            return;
        }

        if let Some(selected_index) = self.selected_profile {
            self.sync_in_progress = true;
            let profile_name = self.profiles[selected_index].clone();
            let manager = Arc::clone(&self.manager);
            let log_tx = self.log_tx.clone();

            self.logs.push_back("---".to_string());
            self.logs.push_back(format!(
                "Starting {}...",
                if dry_run { "Dry Run" } else { "Sync" }
            ));

            // Spawn the blocking I/O in a separate thread
            thread::spawn(move || {
                let result = {
                    let manager_read = manager.read().unwrap();
                    
                    // Get the total number of files
                    let total_files = manager_read.get_profile_link_count(&profile_name);

                    // Send sync started message
                    log_tx.send(WorkerMessage::SyncStarted { total_files }).ok();

                    // Execute sync with progress callback
                    manager_read.execute_sync_with_progress(&profile_name, dry_run, |current, total, file| {
                        log_tx.send(WorkerMessage::Progress {
                            current,
                            total,
                            file: file.to_string(),
                        }).ok();
                    })
                };
                match result {
                    Ok(logs) => {
                        for log in logs {
                            log_tx.send(WorkerMessage::Log(log)).ok();
                        }
                    }
                    Err(e) => {
                        log_tx
                            .send(WorkerMessage::Log(format!("[FATAL ERROR] {}", e)))
                            .ok();
                    }
                }
                // Send a signal that the thread is done
                log_tx.send(WorkerMessage::SyncComplete).ok();
            });
        } else {
            self.logs
                .push_back("[ERROR] No profile selected.".to_string());
        }
    }

    /// Spawns a worker thread to restore a backup.
    ///
    /// # Arguments
    /// * `dry_run` - If true, performs a dry run without making changes
    ///
    /// The restore operation runs in a background thread to keep the UI responsive.
    /// Log messages are sent back via the log channel.
    fn start_restore(&mut self, dry_run: bool) {
        if self.restore_in_progress {
            return;
        }

        if let Some(selected_index) = self.selected_backup {
            if selected_index >= self.backups.len() {
                self.logs
                    .push_back("[ERROR] Invalid backup selection.".to_string());
                return;
            }

            self.restore_in_progress = true;
            let backup = self.backups[selected_index].clone();
            let manager = Arc::clone(&self.manager);
            let log_tx = self.log_tx.clone();

            self.logs.push_back("---".to_string());
            self.logs.push_back(format!(
                "Starting {}...",
                if dry_run {
                    "Restore Dry Run"
                } else {
                    "Restore"
                }
            ));

            // Spawn the blocking I/O in a separate thread
            thread::spawn(move || {
                let result = {
                    let manager_read = manager.read().unwrap();
                    manager_read.restore_backup(&backup, dry_run)
                };
                match result {
                    Ok(logs) => {
                        for log in logs {
                            log_tx.send(WorkerMessage::Log(log)).ok();
                        }
                    }
                    Err(e) => {
                        log_tx
                            .send(WorkerMessage::Log(format!("[FATAL ERROR] {}", e)))
                            .ok();
                    }
                }
                // Send a signal that the thread is done
                log_tx.send(WorkerMessage::RestoreComplete).ok();
            });
        } else {
            self.logs
                .push_back("[ERROR] No backup selected.".to_string());
        }
    }

    /// Deletes the selected backup file.
    ///
    /// This operation runs in a background thread.
    fn delete_selected_backup(&mut self) {
        if self.restore_in_progress {
            return;
        }

        if let Some(selected_index) = self.selected_backup {
            if selected_index >= self.backups.len() {
                self.logs
                    .push_back("[ERROR] Invalid backup selection.".to_string());
                return;
            }

            let backup = self.backups[selected_index].clone();
            let manager = Arc::clone(&self.manager);
            let log_tx = self.log_tx.clone();

            self.logs.push_back("---".to_string());
            self.logs
                .push_back(format!("Deleting backup: {}", backup.backup_path.display()));

            // Spawn the blocking I/O in a separate thread
            thread::spawn(move || {
                {
                    let manager_read = manager.read().unwrap();
                    match manager_read.delete_backup(&backup) {
                        Ok(()) => {
                            log_tx
                                .send(WorkerMessage::Log("[SUCCESS] Backup deleted".to_string()))
                                .ok();
                        }
                        Err(e) => {
                            log_tx
                                .send(WorkerMessage::Log(format!(
                                    "[ERROR] Failed to delete backup: {}",
                                    e
                                )))
                                .ok();
                        }
                    }
                }
                // Refresh the backup list after deletion
                {
                    let manager_read = manager.read().unwrap();
                    match manager_read.list_backups() {
                        Ok(backups) => {
                            log_tx.send(WorkerMessage::BackupsListed(backups)).ok();
                        }
                        Err(e) => {
                            log_tx
                                .send(WorkerMessage::Log(format!(
                                    "[ERROR] Failed to refresh backup list: {}",
                                    e
                                )))
                                .ok();
                        }
                    }
                }
            });
        } else {
            self.logs
                .push_back("[ERROR] No backup selected.".to_string());
        }
    }

    /// Handles key presses in diff preview mode.
    fn handle_diff_mode_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc | KeyCode::Char('q') => self.exit_diff_preview_mode(),
            KeyCode::Char('j') | KeyCode::Down => self.scroll_diff_down(),
            KeyCode::Char('k') | KeyCode::Up => self.scroll_diff_up(),
            KeyCode::Char('n') => self.select_next_diff(),
            KeyCode::Char('N') => self.select_previous_diff(),
            _ => {}
        }
    }

    /// Enters diff preview mode and generates diffs for the selected profile.
    fn enter_diff_preview_mode(&mut self) {
        if let Some(selected_index) = self.selected_profile {
            let profile_name = self.profiles[selected_index].clone();
            self.mode = AppMode::DiffPreview;
            self.diff_scroll = 0;
            self.logs.push_back("---".to_string());
            self.logs
                .push_back(format!("Generating diff preview for profile: {}", profile_name));

            let manager = Arc::clone(&self.manager);
            let log_tx = self.log_tx.clone();

            // Spawn a thread to generate diffs
            thread::spawn(move || match manager.preview_diff(&profile_name) {
                Ok(diffs) => {
                    log_tx.send(WorkerMessage::DiffGenerated(diffs)).ok();
                }
                Err(e) => {
                    log_tx
                        .send(WorkerMessage::Log(format!(
                            "[ERROR] Failed to generate diff: {}",
                            e
                        )))
                        .ok();
                    log_tx.send(WorkerMessage::DiffGenerated(Vec::new())).ok();
                }
            });
        } else {
            self.logs
                .push_back("[ERROR] No profile selected.".to_string());
        }
    }

    /// Exits diff preview mode and returns to sync mode.
    fn exit_diff_preview_mode(&mut self) {
        self.mode = AppMode::Sync;
        self.diffs.clear();
        self.selected_diff = None;
        self.diff_scroll = 0;
        self.logs.push_back("---".to_string());
        self.logs.push_back("Exited Diff Preview Mode".to_string());
    }

    /// Scrolls down in the diff view.
    fn scroll_diff_down(&mut self) {
        self.diff_scroll = self.diff_scroll.saturating_add(1);
    }

    /// Scrolls up in the diff view.
    fn scroll_diff_up(&mut self) {
        self.diff_scroll = self.diff_scroll.saturating_sub(1);
    }

    /// Selects the next diff in the list.
    fn select_next_diff(&mut self) {
        if self.diffs.is_empty() {
            return;
        }
        let i = self.selected_diff.unwrap_or(0);
        let next = if i >= self.diffs.len() - 1 {
            0
        } else {
            i + 1
        };
        self.selected_diff = Some(next);
        self.diff_scroll = 0; // Reset scroll when switching diffs
    }

    /// Selects the previous diff in the list.
    fn select_previous_diff(&mut self) {
        if self.diffs.is_empty() {
            return;
        }
        let i = self.selected_diff.unwrap_or(0);
        let prev = if i == 0 {
            self.diffs.len() - 1
        } else {
            i - 1
        };
        self.selected_diff = Some(prev);
        self.diff_scroll = 0; // Reset scroll when switching diffs
    }

    /// Called when the app receives a new message from a worker thread.
    ///
    /// # Arguments
    /// * `msg` - The message received from a worker thread
    ///
    /// Handles log messages and sync completion signals.
    /// Implements log rotation to prevent unbounded memory growth.
    pub fn on_log(&mut self, msg: WorkerMessage) {
        match msg {
            WorkerMessage::Log(log) => {
                self.logs.push_back(log);
                // Implement log rotation to limit memory usage
                while self.logs.len() > MAX_LOG_MESSAGES {
                    self.logs.pop_front();
                }
            }
            WorkerMessage::SyncComplete => {
                self.sync_in_progress = false;
                self.sync_progress = None;
            }
            WorkerMessage::RestoreComplete => {
                self.restore_in_progress = false;
                // Refresh backup list after restore
                self.enter_restore_mode();
            }
            WorkerMessage::BackupsListed(backups) => {
                let count = backups.len();
                self.backups = backups;
                self.selected_backup = if count > 0 { Some(0) } else { None };
                self.logs.push_back(format!("Found {} backup(s)", count));
            }
            WorkerMessage::DiffGenerated(diffs) => {
                let count = diffs.len();
                self.diffs = diffs;
                self.selected_diff = if count > 0 { Some(0) } else { None };
                self.logs.push_back(format!("Generated {} diff(s)", count));
            }
        }
    }
}
