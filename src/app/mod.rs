//! Application state and event loop. No business logic; coordinates disk/flash and UI.

mod events;
pub mod state;

pub use events::run_event_loop;
pub use state::{AppState, Screen};
