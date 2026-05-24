//! Docking layout system for iced.
//!
//! ## Serialization
//!
//! Enable the `serde` feature to derive [`serde::Serialize`] and [`serde::Deserialize`] on layout
//! types. Persist declarative [`LayoutTree`] for defaults, or [`Layout`] to capture runtime
//! split/tab state after user edits.

pub mod builder;
pub mod error;
pub mod factory;
pub mod manager;
pub mod model;
pub mod style;
pub mod widget;

pub use builder::{
    build_tree, horizontal, panel, single, tabs, vertical, BuiltLayout, DockIndex, DockSession,
    LayoutTree, PaneTarget, PanelDef, SplitNode, TabsNode,
};
pub use error::{Error, Result};
pub use factory::Factory;
pub use manager::{DockManager, DragSession, DropZone};
pub use model::{
    Axis, ContentKey, DockOperation, Layout, NodeEntry, NodeId, NodeKind, Pane, Panel,
    ProportionalGroup,
};
pub use style::{
    close_button_style, constant, CloseButtonStyle, DockBackgroundStyle, DockStyle,
    DropOverlayStyle, SplitterStyle, TabBarStyle, TabStyle, WindowStyle,
};
pub use widget::{
    apply_message, dock, finish_drag, handle_dock_message, Dock, DockMessage, DockWidgetState,
    TabMessage,
};
