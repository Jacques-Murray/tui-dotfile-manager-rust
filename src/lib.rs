// Author: Jacques Murray
//! Library crate for the TUI Dotfile Manager.
//! Exposes core functionality for integration tests and potential library usage.

pub mod core;

// Re-export commonly used types for convenience
pub use core::error::ManagerError;
pub use core::manager::DotfileManager;
