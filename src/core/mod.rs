// Author: Jacques Murray
//! Core module: contains all the business logic.

pub mod config;
pub mod error;
pub mod manager;

// Publicly export the main components for the binary to use.
pub use config::Config;
pub use error::ManagerError;
pub use manager::DotfileManager;
