//! Docking layout system for iced.

pub mod factory;
pub mod manager;
pub mod model;
pub mod widget;

pub use factory::Factory;
pub use manager::{DockManager, DragSession, DropZone};
pub use model::{
    Axis, ContentKey, DockOperation, DockableMeta, Layout, NodeEntry, NodeId, NodeKind,
    ProportionalGroup, TabGroup,
};
pub use widget::{
    apply_message, dock, handle_dock_message, Dock, DockMessage, DockWidgetState, TabMessage,
};
