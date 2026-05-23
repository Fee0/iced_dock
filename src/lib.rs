//! Docking layout system for iced.

pub mod factory;
pub mod manager;
pub mod model;
pub mod widget;

pub use factory::Factory;
pub use manager::{DockManager, DragSession, DropZone};
pub use model::{
    Axis, ContentKey, DockOperation, Layout, NodeEntry, NodeId, NodeKind, Panel, Pane,
    ProportionalGroup,
};
pub use widget::{
    apply_message, dock, finish_drag, handle_dock_message, Dock, DockMessage, DockWidgetState,
    TabMessage,
};
