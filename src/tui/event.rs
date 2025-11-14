// Author: Jacques Murray
//! TUI Event handler thread.

use crossterm::event::{self, Event as CrosstermEvent, KeyCode, KeyEventKind};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

/// Tick rate for the event loop in milliseconds.
/// Controls how frequently the UI updates and checks for events.
const TICK_RATE_MS: u64 = 250;

/// Represents an application event.
#[derive(Debug)]
pub enum Event {
    /// A key was pressed
    Key(KeyCode),
    /// A tick event for periodic updates
    Tick,
    /// An error occurred in the event thread
    Error(String),
}

/// Runs the event listener in a separate thread.
/// 
/// Sends events back to the main loop via a channel. This includes keyboard
/// input and periodic tick events for UI updates.
/// 
/// # Arguments
/// * `tx` - Channel sender for transmitting events to the main thread
/// 
/// # Errors
/// Returns an error if the thread cannot be spawned (rare).
/// Runtime errors in the event thread are sent as Event::Error variants.
pub fn run(tx: mpsc::Sender<Event>) -> anyhow::Result<()> {
    let tick_rate = Duration::from_millis(TICK_RATE_MS);
    thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            match event::poll(timeout) {
                Ok(true) => {
                    match event::read() {
                        Ok(CrosstermEvent::Key(key)) => {
                            // Only send key press events
                            if key.kind == KeyEventKind::Press
                                && tx.send(Event::Key(key.code)).is_err()
                            {
                                // Main thread has dropped the receiver, exit gracefully
                                break;
                            }
                        }
                        Err(e) => {
                            let _ = tx.send(Event::Error(format!("Failed to read event: {}", e)));
                            break;
                        }
                        _ => {} // Ignore other event types
                    }
                }
                Ok(false) => {} // No event available
                Err(e) => {
                    let _ = tx.send(Event::Error(format!("Failed to poll events: {}", e)));
                    break;
                }
            }

            if last_tick.elapsed() >= tick_rate {
                if tx.send(Event::Tick).is_err() {
                    // Main thread has dropped the receiver, exit gracefully
                    break;
                }
                last_tick = Instant::now();
            }
        }
    });
    Ok(())
}
