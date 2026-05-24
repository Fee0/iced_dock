//! Custom iced widgets for docking.

mod action;
mod compose;
mod dock;
mod event;
mod split;
mod tab_dock;
mod tab_strip;

pub use action::{DockAction, TabAction};
pub use dock::{dispatch_action, dock, finish_drag, Dock, DockBuilder, DockWidgetState};
pub use event::DockEvent;
