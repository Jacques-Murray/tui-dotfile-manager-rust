// Author: Jacques Murray
//! TUI module: contains all logic for the Ratatui interface.

pub mod app;
pub mod event;
pub mod ui;

// Publicly export the main components for the binary to use.
pub use app::App;
pub use event::{run as event_listener, Event};
pub use ui::render;
