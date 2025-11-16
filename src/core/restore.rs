// Author: Jacques Murray
//! Backup restoration logic and backup entry management.

use chrono::{DateTime, Local, NaiveDateTime, TimeZone};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Represents a single backed-up file with metadata.
///
/// BackupEntry contains all information needed to display, restore,
/// or delete a backup file.
#[derive(Debug, Clone)]
pub struct BackupEntry {
    /// Original filename (e.g., ".bashrc")
    pub original_name: String,
    /// Timestamp when the backup was created
    pub timestamp: DateTime<Local>,
    /// Full path to the backup file
    pub backup_path: PathBuf,
    /// Expected target path for restoration
    pub target_path: PathBuf,
    /// Size of the backup file in bytes
    pub file_size: u64,
}

impl BackupEntry {
    /// Parses a backup filename to extract the original name and timestamp.
    ///
    /// # Arguments
    /// * `filename` - Backup filename (e.g., ".bashrc_20241114_143052.123456")
    ///
    /// # Returns
    /// Some((original_name, timestamp)) if parsing succeeds, None otherwise
    ///
    /// # Example
    /// ```
    /// use tui_dotfile_manager::core::restore::BackupEntry;
    ///
    /// let result = BackupEntry::parse_backup_filename(".bashrc_20241114_143052.123456");
    /// assert!(result.is_some());
    /// ```
    pub fn parse_backup_filename(filename: &str) -> Option<(String, DateTime<Local>)> {
        // Expected format: <original_name>_<YYYYMMDD>_<HHMMSS>.<microseconds>
        // Example: .bashrc_20241114_143052.123456

        // Find the last underscore before the time component
        let parts: Vec<&str> = filename.rsplitn(2, '_').collect();
        if parts.len() != 2 {
            return None;
        }

        let time_part = parts[0]; // "143052.123456"
        let remaining = parts[1]; // ".bashrc_20241114"

        // Find the second-to-last underscore to separate date
        let parts2: Vec<&str> = remaining.rsplitn(2, '_').collect();
        if parts2.len() != 2 {
            return None;
        }

        let date_part = parts2[0]; // "20241114"
        let original_name = parts2[1]; // ".bashrc"

        // Parse timestamp: date_part + time_part
        let timestamp_str = format!("{}_{}", date_part, time_part);

        // Parse format: YYYYMMDD_HHMMSS.microseconds
        let dt = NaiveDateTime::parse_from_str(&timestamp_str, "%Y%m%d_%H%M%S%.6f").ok()?;
        let local_dt = Local.from_local_datetime(&dt).single()?;

        Some((original_name.to_string(), local_dt))
    }

    /// Creates a BackupEntry from a backup file path.
    ///
    /// # Arguments
    /// * `backup_path` - Path to the backup file
    /// * `target_base` - Base directory for resolving target paths (e.g., home directory)
    ///
    /// # Returns
    /// Some(BackupEntry) if the file is a valid backup, None otherwise
    #[allow(dead_code)]
    pub fn from_path(backup_path: &Path, target_base: &Path) -> Option<Self> {
        let filename = backup_path.file_name()?.to_str()?;
        let (original_name, timestamp) = Self::parse_backup_filename(filename)?;

        // Get file metadata
        let metadata = fs::metadata(backup_path).ok()?;
        let file_size = metadata.len();

        // Construct expected target path
        let target_path = target_base.join(&original_name);

        Some(BackupEntry {
            original_name,
            timestamp,
            backup_path: backup_path.to_path_buf(),
            target_path,
            file_size,
        })
    }

    /// Formats the file size in a human-readable format.
    pub fn format_size(&self) -> String {
        let size = self.file_size;
        if size < 1024 {
            format!("{} B", size)
        } else if size < 1024 * 1024 {
            format!("{:.1} KB", size as f64 / 1024.0)
        } else if size < 1024 * 1024 * 1024 {
            format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
        } else {
            format!("{:.1} GB", size as f64 / (1024.0 * 1024.0 * 1024.0))
        }
    }

    /// Reads and returns the first N lines of the backup file.
    ///
    /// # Arguments
    /// * `max_lines` - Maximum number of lines to read
    ///
    /// # Returns
    /// A vector of strings, one per line, or an error if reading fails
    pub fn preview_content(&self, max_lines: usize) -> io::Result<Vec<String>> {
        let content = fs::read_to_string(&self.backup_path)?;
        Ok(content
            .lines()
            .take(max_lines)
            .map(|s| s.to_string())
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_backup_filename_valid() {
        let filename = ".bashrc_20241114_143052.123456";
        let result = BackupEntry::parse_backup_filename(filename);

        assert!(result.is_some());
        let (name, _timestamp) = result.unwrap();
        assert_eq!(name, ".bashrc");
    }

    #[test]
    fn test_parse_backup_filename_with_underscores() {
        let filename = "my_config_file_20241114_143052.123456";
        let result = BackupEntry::parse_backup_filename(filename);

        assert!(result.is_some());
        let (name, _timestamp) = result.unwrap();
        assert_eq!(name, "my_config_file");
    }

    #[test]
    fn test_parse_backup_filename_invalid() {
        let invalid_filenames = vec![
            "invalid",
            ".bashrc_invalid_timestamp",
            "no_timestamp",
            ".bashrc_20241114",
        ];

        for filename in invalid_filenames {
            assert!(
                BackupEntry::parse_backup_filename(filename).is_none(),
                "Should fail for: {}",
                filename
            );
        }
    }

    #[test]
    fn test_format_size() {
        let entry = BackupEntry {
            original_name: "test".to_string(),
            timestamp: Local::now(),
            backup_path: PathBuf::from("/tmp/test"),
            target_path: PathBuf::from("/tmp/target"),
            file_size: 1024,
        };

        assert_eq!(entry.format_size(), "1.0 KB");
    }
}
