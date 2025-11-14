// Author: Jacques Murray
//! Core module: contains all the business logic.

pub mod config;
pub mod error;
pub mod manager;

// Publicly export the main components for the binary to use.
#[allow(unused_imports)]
pub use error::ManagerError;
pub use manager::DotfileManager;
