//! Docking layout system for iced.

pub mod builder;
pub mod factory;
pub mod manager;
pub mod model;
pub mod style;
pub mod widget;

pub use builder::{
    build_tree, horizontal, panel, single, tabs, vertical, BuiltLayout, DockIndex, DockSession,
    LayoutError, LayoutTree, PanelDef, PaneTarget, SplitNode, TabsNode,
};
pub use factory::Factory;
pub use manager::{DockManager, DragSession, DropZone};
pub use model::{
    Axis, ContentKey, DockOperation, Layout, NodeEntry, NodeId, NodeKind, Panel, Pane,
    ProportionalGroup,
};
pub use style::{
    close_button_style, constant, tab_button_style, CloseButtonStyle, DockBackgroundStyle,
    DockStyle, DropOverlayStyle, SplitterStyle, TabBarStyle, TabStyle, TitleBarStyle, WindowStyle,
};
pub use widget::{
    apply_message, dock, finish_drag, handle_dock_message, Dock, DockMessage, DockWidgetState,
    TabMessage,
};
