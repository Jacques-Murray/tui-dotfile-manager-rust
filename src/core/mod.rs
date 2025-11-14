// Author: Jacques Murray
//! Core module: contains all the business logic.

pub mod config;
pub mod error;
pub mod manager;

// Publicly export the main components for library usage.
// ManagerError is re-exported for external crates that might use this as a library.
#[allow(unused_imports)]
pub use error::ManagerError;
pub use manager::DotfileManager;
