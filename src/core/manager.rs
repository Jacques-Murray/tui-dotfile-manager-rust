// Author: Jacques Murray
//! The main DotfileManager struct and its associated logic.

use super::config::Config;
use super::error::ManagerError;
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
    /// let manager = DotfileManager::new(Path::new("config.toml"))?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new(config_path: &Path) -> Result<Self, ManagerError> {
        if !config_path.exists() {
            return Err(ManagerError::ConfigNotFound(config_path.to_path_buf()));
        }

        let config_str = fs::read_to_string(config_path)?;
        let config: Config = toml::from_str(&config_str)?;
        
        // Validate configuration
        if let Err(e) = config.validate() {
            return Err(ManagerError::Io(io::Error::new(
                io::ErrorKind::InvalidData,
                e
            )));
        }

        let config_dir = config_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();

        // Resolve paths relative to the config file or home dir
        let repo_path = Self::resolve_path(&config_dir, &config.settings.repo_dir)?;
        let backup_path = Self::resolve_path(&config_dir, &config.settings.backup_dir)?;

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
            let source = Self::resolve_path(&self.repo_path, &link.source)?;
            let target = Self::resolve_path(&self.config_dir, &link.target)?;

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
                logs.push(format!(
                    "         Expected at: {}",
                    source.display()
                ));
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

        // Check if target exists - use metadata directly to avoid TOCTOU
        if let Ok(metadata) = target.symlink_metadata() {
            if metadata.is_symlink() {
                let existing_link = fs::read_link(target)?;
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
                        fs::remove_file(target)?;
                    }
                }
            } else {
                // It's a real file or directory - include microseconds to prevent collisions
                let ts = Local::now().format("%Y%m%d_%H%M%S%.6f");
                let backup_name = format!(
                    "{}_{}",
                    target.file_name().unwrap_or_default().to_string_lossy(),
                    ts
                );
                let backup_path = self.backup_path.join(backup_name);

                logs.push(format!(
                    "  [BACKUP] Moving existing file: {} -> {}",
                    target.display(),
                    backup_path.display()
                ));
                if !dry_run {
                    fs::rename(target, backup_path)?;
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
    fn resolve_path(base: &Path, p: &Path) -> Result<PathBuf, ManagerError> {
        let expanded = shellexpand::path::tilde(p);

        if expanded.is_absolute() {
            Ok(expanded.to_path_buf())
        } else {
            Ok(base.join(expanded))
        }
    }
}
