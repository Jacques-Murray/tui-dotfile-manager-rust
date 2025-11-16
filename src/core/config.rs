// Author: Jacques Murray
//! Defines the strongly-typed configuration structs using Serde.

use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

/// The root configuration struct, mapping to the TOML file.
///
/// # Example TOML
/// ```toml
/// [settings]
/// repo_dir = "dotfiles"
/// backup_dir = "~/.dotfile_backups"
///
/// [profiles.personal]
/// links = [
///   { source = ".bashrc", target = "~/.bashrc" },
/// ]
/// ```
#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub settings: Settings,
    pub profiles: HashMap<String, Profile>,
}

impl Config {
    /// Validates the configuration to ensure all required fields are present
    /// and that profile names are non-empty.
    pub fn validate(&self) -> Result<(), String> {
        if self.profiles.is_empty() {
            return Err("Configuration must contain at least one profile".to_string());
        }

        for (name, profile) in &self.profiles {
            if name.is_empty() {
                return Err("Profile names cannot be empty".to_string());
            }
            if profile.links.is_empty() {
                return Err(format!("Profile '{}' has no links defined", name));
            }
        }

        Ok(())
    }
}

/// The [settings] section of the config.
///
/// Contains global settings for the dotfile manager including
/// the repository directory and backup location.
#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    /// Directory containing the dotfiles (relative to config file or absolute)
    pub repo_dir: PathBuf,
    /// Directory where backups of replaced files will be stored (supports ~ expansion)
    pub backup_dir: PathBuf,
}

/// A single profile, e.g., [profiles.work]
///
/// A profile represents a collection of symlink operations that can be
/// executed together. Useful for maintaining different dotfile sets for
/// different environments (work, personal, etc.).
#[derive(Debug, Deserialize, Clone)]
pub struct Profile {
    /// List of symlink operations to perform for this profile
    pub links: Vec<Link>,
}

/// A single link task within a profile.
///
/// Represents a symlink operation from a source file in the repository
/// to a target location on the filesystem.
#[derive(Debug, Deserialize, Clone)]
pub struct Link {
    /// Source file path (relative to repo_dir)
    pub source: PathBuf,
    /// Target path where the symlink will be created (supports ~ expansion)
    pub target: PathBuf,
}
