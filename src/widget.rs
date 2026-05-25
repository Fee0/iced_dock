//! Custom iced widgets for docking.

mod action;
mod compose;
mod dock;
mod event;
mod split;
mod state;
mod tab_dock;
mod tab_strip;

pub use action::{DockAction, TabAction};
pub use dock::{dock, Dock, DockBuilder};
pub use state::{dispatch_action, finish_drag, DockWidgetState};
pub use event::DockEvent;
pub use crate::style::PaneContent;
