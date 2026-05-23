//! Custom iced widgets for docking.

mod compose;
mod dock;
mod message;
mod split;
mod tab_dock;

pub use dock::{apply_message, dock, finish_drag, handle_dock_message, Dock, DockWidgetState};
pub use message::{DockMessage, TabMessage};
