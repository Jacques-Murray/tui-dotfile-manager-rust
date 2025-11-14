// Author: Jacques Murray
//! Defines the custom error type for the core logic.

use std::io;
use std::path::PathBuf;
use thiserror::Error;

/// A custom error enum for all possible failures in the DotfileManager.
/// 
/// This enum uses `thiserror` to provide descriptive error messages and
/// automatic conversion from standard library error types.
#[derive(Debug, Error)]
pub enum ManagerError {
    #[error("Configuration file not found at: {0}")]
    ConfigNotFound(PathBuf),

    #[error("Failed to parse configuration file: {0}")]
    ConfigParse(#[from] toml::de::Error),

    #[error("File I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("Profile not found in configuration: {0}")]
    ProfileNotFound(String),
}
