// Author: Jacques Murray
//! TUI Event handler thread.

use crossterm::event::{self, Event as CrosstermEvent, KeyCode, KeyEventKind};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

/// Represents an application event.
#[derive(Debug)]
pub enum Event {
    Key(KeyCode),
    Tick,
}

/// Runs the event listener in a separate thread.
/// Sends events back to the main loop via a channel.
pub fn run(tx: mpsc::Sender<Event>) -> anyhow::Result<()> {
    let tick_rate = Duration::from_millis(250);
    thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout).expect("Failed to poll events") {
                if let CrosstermEvent::Key(key) = event::read().expect("Failed to read event") {
                    // Only send key press events
                    if key.kind == KeyEventKind::Press {
                        tx.send(Event::Key(key.code))
                            .expect("Failed to send key event");
                    }
                }
            }

            if last_tick.elapsed() >= tick_rate {
                tx.send(Event::Tick).expect("Failed to send tick event");
                last_tick = Instant::now();
            }
        }
    });
    Ok(())
}
