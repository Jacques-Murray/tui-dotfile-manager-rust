// Author: Jacques Murray
//! The main DotfileManager struct and its associated logic.

use super::config::Config;
use super::diff::{generate_diff, DiffResult};
use super::error::ManagerError;
use super::restore::BackupEntry;
use chrono::Local;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Encapsulates all dotfile management logic.
///
/// The manager is responsible for loading configuration, resolving paths,
/// and executing sync operations. It is stateless after initialization
/// and can be safely shared across threads.
#[derive(Debug, Clone)]
pub struct DotfileManager {
    config: Config,
    repo_path: PathBuf,
    backup_path: PathBuf,
    config_dir: PathBuf,
}

impl DotfileManager {
    /// Creates a new manager by loading and parsing the config file.
    ///
    /// # Arguments
    /// * `config_path` - Path to the TOML configuration file
    ///
    /// # Errors
    /// Returns an error if:
    /// - The configuration file doesn't exist
    /// - The file cannot be read
    /// - The TOML is invalid
    /// - The configuration fails validation
    ///
    /// # Example
    /// ```no_run
    /// use std::path::Path;
    /// use tui_dotfile_manager::DotfileManager;
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let manager = DotfileManager::new(Path::new("config.toml"))?;
    ///     Ok(())
    /// }
    /// ```
    pub fn new(config_path: &Path) -> Result<Self, ManagerError> {
        if !config_path.exists() {
            return Err(ManagerError::ConfigNotFound(config_path.to_path_buf()));
        }

        let config_str = fs::read_to_string(config_path)?;
        let config: Config = toml::from_str(&config_str)?;

        // Validate configuration
        if let Err(e) = config.validate() {
            return Err(ManagerError::ConfigValidation(e));
        }

        let config_dir = config_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();

        // Resolve paths relative to the config file or home dir
        let repo_path = Self::resolve_path(&config_dir, &config.settings.repo_dir);
        let backup_path = Self::resolve_path(&config_dir, &config.settings.backup_dir);

        Ok(Self {
            config,
            repo_path,
            backup_path,
            config_dir,
        })
    }

    /// Returns a sorted list of available profile names.
    ///
    /// Profile names are sorted alphabetically for consistent display
    /// in the user interface.
    ///
    /// # Example
    /// ```no_run
    /// # use std::path::Path;
    /// # use tui_dotfile_manager::DotfileManager;
    /// # let manager = DotfileManager::new(Path::new("config.toml"))?;
    /// let profiles = manager.get_profiles();
    /// println!("Available profiles: {:?}", profiles);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn get_profiles(&self) -> Vec<String> {
        let mut profiles: Vec<String> = self.config.profiles.keys().cloned().collect();
        profiles.sort();
        profiles
    }

    /// Executes the sync, either for real or as a dry run.
    /// Returns a list of log messages.
    pub fn execute_sync(
        &self,
        profile_name: &str,
        dry_run: bool,
    ) -> Result<Vec<String>, ManagerError> {
        let mut logs = Vec::new();

        if dry_run {
            logs.push("--- DRY RUN (No changes will be made) ---".to_string());
        } else {
            logs.push(format!(
                "--- Executing Sync for profile: {profile_name} ---"
            ));
            fs::create_dir_all(&self.backup_path)?;
            logs.push(format!("Backup directory ensured: {:?}", self.backup_path));
        }

        let profile = self
            .config
            .profiles
            .get(profile_name)
            .ok_or_else(|| ManagerError::ProfileNotFound(profile_name.to_string()))?;

        for link in &profile.links {
            let source = Self::resolve_path(&self.repo_path, &link.source);
            let target = Self::resolve_path(&self.config_dir, &link.target);

            logs.push(format!(
                "Processing: {} -> {}",
                source
                    .strip_prefix(&self.repo_path)
                    .unwrap_or(&source)
                    .display(),
                target.display()
            ));

            if !source.exists() {
                logs.push(format!(
                    "  [WARN] Source file does not exist: {}",
                    link.source.display()
                ));
                logs.push(format!("         Expected at: {}", source.display()));
                continue;
            }

            self.handle_target(&source, &target, dry_run, &mut logs)?;
        }

        logs.push("--- Sync Finished ---".to_string());
        Ok(logs)
    }

    /// Handles the logic for an individual target file/symlink.
    fn handle_target(
        &self,
        source: &Path,
        target: &Path,
        dry_run: bool,
        logs: &mut Vec<String>,
    ) -> Result<(), io::Error> {
        let mut needs_link = true;

        // Attempt to handle existing target to avoid TOCTOU issues
        if target.exists() {
            match fs::read_link(target) {
                Ok(existing_link) => {
                    if existing_link == source {
                        logs.push(format!(
                            "  [SKIP] Link already correct: {}",
                            target.display()
                        ));
                        needs_link = false;
                    } else {
                        logs.push(format!(
                            "  [BACKUP] Removing incorrect symlink: {}",
                            target.display()
                        ));
                        if !dry_run {
                            // Ignore error if file is already gone
                            let _ = fs::remove_file(target);
                        }
                    }
                }
                Err(_) => {
                    // Not a symlink, treat as file or directory
                    let ts = Local::now().format("%Y%m%d_%H%M%S%.6f");
                    let file_name = target.file_name().ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::InvalidInput,
                            format!("Target path has no filename: {}", target.display()),
                        )
                    })?;
                    let backup_name = format!("{}_{}", file_name.to_string_lossy(), ts);
                    let backup_path = self.backup_path.join(backup_name);

                    logs.push(format!(
                        "  [BACKUP] Moving existing file: {} -> {}",
                        target.display(),
                        backup_path.display()
                    ));
                    if !dry_run {
                        // Ignore error if file is already gone
                        let _ = fs::rename(target, backup_path);
                    }
                }
            }
        }

        // Create the link if needed
        if needs_link {
            logs.push(format!(
                "  [LINK] Creating symlink: {} -> {}",
                source.display(),
                target.display()
            ));
            if !dry_run {
                // Ensure parent directory exists
                if let Some(parent) = target.parent() {
                    if !parent.exists() {
                        fs::create_dir_all(parent)?;
                    }
                }
                // Use platform-specific symlink functions
                self.create_symlink(source, target)?;
            }
        }
        Ok(())
    }

    /// Creates a platform-aware symlink (file vs. dir on Windows).
    ///
    /// On Windows, different functions are needed for file and directory symlinks.
    #[cfg(windows)]
    fn create_symlink(&self, source: &Path, target: &Path) -> Result<(), io::Error> {
        if source.is_dir() {
            std::os::windows::fs::symlink_dir(source, target)
        } else {
            std::os::windows::fs::symlink_file(source, target)
        }
    }

    /// Creates a platform-aware symlink (Unix).
    ///
    /// On Unix-like systems, a single function handles both files and directories.
    #[cfg(not(windows))]
    fn create_symlink(&self, source: &Path, target: &Path) -> Result<(), io::Error> {
        std::os::unix::fs::symlink(source, target)
    }

    /// Helper to expand '~' and resolve paths.
    ///
    /// # Arguments
    /// * `base` - Base directory for relative paths
    /// * `p` - Path to resolve (may contain ~ or be relative/absolute)
    ///
    /// # Behavior
    /// - Expands ~ to the user's home directory
    /// - If the path is absolute (after expansion), returns it as-is
    /// - If relative, joins it with the base directory
    fn resolve_path(base: &Path, p: &Path) -> PathBuf {
        let expanded = shellexpand::path::tilde(p);

        if expanded.is_absolute() {
            expanded.to_path_buf()
        } else {
            base.join(expanded)
        }
    }

    /// Lists all backup files in the backup directory.
    ///
    /// Returns a vector of BackupEntry structs sorted by timestamp (newest first).
    /// Each entry contains metadata about the backup including original filename,
    /// timestamp, size, and target path.
    ///
    /// # Returns
    /// A Result containing a vector of BackupEntry or ManagerError
    ///
    /// # Errors
    /// Returns an error if the backup directory cannot be read
    pub fn list_backups(&self) -> Result<Vec<BackupEntry>, ManagerError> {
        // If backup directory doesn't exist, return empty list
        if !self.backup_path.exists() {
            return Ok(Vec::new());
        }

        let mut backups = Vec::new();

        // Read all files from backup directory
        for entry in fs::read_dir(&self.backup_path)? {
            let entry = entry?;
            let path = entry.path();

            // Skip directories, only process files
            if !path.is_file() {
                continue;
            }

            // Parse backup filename
            if let Some(filename_str) = path.file_name().and_then(|s| s.to_str()) {
                if let Some((original_name, timestamp)) =
                    BackupEntry::parse_backup_filename(filename_str)
                {
                    // Get file metadata
                    if let Ok(metadata) = fs::metadata(&path) {
                        // Find matching target from config by looking through all profiles
                        let mut target_path = None;
                        for profile in self.config.profiles.values() {
                            for link in &profile.links {
                                let link_target =
                                    Self::resolve_path(&self.config_dir, &link.target);
                                if let Some(target_filename) = link_target.file_name() {
                                    if target_filename.to_string_lossy() == original_name {
                                        target_path = Some(link_target);
                                        break;
                                    }
                                }
                            }
                            if target_path.is_some() {
                                break;
                            }
                        }

                        // If no matching config entry, default to home directory
                        let target_path = target_path.unwrap_or_else(|| {
                            let home_dir = shellexpand::path::tilde("~");
                            home_dir.join(&original_name)
                        });

                        backups.push(BackupEntry {
                            original_name,
                            timestamp,
                            backup_path: path.to_path_buf(),
                            target_path,
                            file_size: metadata.len(),
                        });
                    }
                }
            }
        }

        // Sort by timestamp, newest first
        backups.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        Ok(backups)
    }

    /// Restores a backup file to its original target location.
    ///
    /// # Arguments
    /// * `backup` - The BackupEntry to restore
    /// * `dry_run` - If true, only simulates the restore without making changes
    ///
    /// # Returns
    /// A Result containing log messages or ManagerError
    ///
    /// # Behavior
    /// - Checks if target location exists
    /// - If target exists, backs it up before restoring (unless dry_run)
    /// - Copies the backup file to the target location
    /// - Removes the backup file from the backup directory
    pub fn restore_backup(
        &self,
        backup: &BackupEntry,
        dry_run: bool,
    ) -> Result<Vec<String>, ManagerError> {
        let mut logs = Vec::new();

        if dry_run {
            logs.push("--- DRY RUN (No changes will be made) ---".to_string());
        } else {
            logs.push(format!(
                "--- Restoring backup: {} ---",
                backup.original_name
            ));
        }

        logs.push(format!("Backup file: {}", backup.backup_path.display()));
        logs.push(format!("Target location: {}", backup.target_path.display()));

        // Check if backup file still exists
        if !backup.backup_path.exists() {
            logs.push("[ERROR] Backup file no longer exists".to_string());
            return Ok(logs);
        }

        // Check if target location exists (including broken symlinks)
        // Use symlink_metadata to detect symlinks even if they're broken
        match fs::symlink_metadata(&backup.target_path) {
            Ok(metadata) => {
                if metadata.is_symlink() {
                    logs.push("[INFO] Target is a symlink, will be removed".to_string());
                    if !dry_run {
                        fs::remove_file(&backup.target_path)?;
                    }
                } else {
                    // Regular file/directory - back it up
                    let ts = Local::now().format("%Y%m%d_%H%M%S%.6f");
                    let file_name = backup.target_path.file_name().ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::InvalidInput,
                            format!(
                                "Target path has no filename: {}",
                                backup.target_path.display()
                            ),
                        )
                    })?;
                    let backup_name = format!("{}_{}", file_name.to_string_lossy(), ts);
                    let new_backup_path = self.backup_path.join(backup_name);

                    logs.push(format!(
                        "[BACKUP] Backing up existing file: {} -> {}",
                        backup.target_path.display(),
                        new_backup_path.display()
                    ));
                    if !dry_run {
                        fs::rename(&backup.target_path, new_backup_path)?;
                    }
                }
            }
            Err(_) => {
                // Target does not exist
                logs.push("[INFO] Target does not exist, will be created".to_string());
            }
        }

        // Ensure parent directory exists
        if let Some(parent) = backup.target_path.parent() {
            if !parent.exists() {
                logs.push(format!(
                    "[CREATE] Creating parent directory: {}",
                    parent.display()
                ));
                if !dry_run {
                    fs::create_dir_all(parent)?;
                }
            }
        }

        // Copy backup to target location
        logs.push(format!(
            "[RESTORE] Copying backup to target: {}",
            backup.target_path.display()
        ));
        if !dry_run {
            fs::copy(&backup.backup_path, &backup.target_path)?;
        }

        // Remove the backup file
        logs.push(format!(
            "[CLEANUP] Removing backup file: {}",
            backup.backup_path.display()
        ));
        if !dry_run {
            fs::remove_file(&backup.backup_path)?;
        }

        logs.push("--- Restore Finished ---".to_string());
        Ok(logs)
    }

    /// Deletes a backup file from the backup directory.
    ///
    /// # Arguments
    /// * `backup` - The BackupEntry to delete
    ///
    /// # Returns
    /// A Result indicating success or ManagerError
    ///
    /// # Errors
    /// Returns an error if the file cannot be deleted
    pub fn delete_backup(&self, backup: &BackupEntry) -> Result<(), ManagerError> {
        if backup.backup_path.exists() {
            fs::remove_file(&backup.backup_path)?;
        }
        Ok(())
    }

    /// Generates diff previews for all links in a profile.
    ///
    /// # Arguments
    /// * `profile_name` - Name of the profile to preview
    ///
    /// # Returns
    /// A vector of DiffResult for each link in the profile
    ///
    /// # Errors
    /// Returns an error if the profile is not found
    pub fn preview_diff(&self, profile_name: &str) -> Result<Vec<DiffResult>, ManagerError> {
        let profile = self
            .config
            .profiles
            .get(profile_name)
            .ok_or_else(|| ManagerError::ProfileNotFound(profile_name.to_string()))?;

        let mut results = Vec::new();

        for link in &profile.links {
            let source = Self::resolve_path(&self.repo_path, &link.source);
            let target = Self::resolve_path(&self.config_dir, &link.target);

            let diff = generate_diff(&source, &target);
            results.push(diff);
        }

        Ok(results)
    }
}
