//! Docking layout system for [iced](https://github.com/iced-rs/iced).
//!
//! Model-first API inspired by [Dock](https://github.com/wieslawsoltes/Dock):
//! layout tree, factory mutations, drag manager, and custom iced widgets.

pub mod factory;
pub mod manager;
pub mod model;
pub mod state;
pub mod view;
pub mod widget;

pub use factory::{default_ide_layout, Factory, FactoryError, FactoryResult};
pub use manager::{
    hit_test_drop_zone, DockError, DockManager, DragSession, DropZone,
};
pub use model::{
    Axis, ContentKey, DockOperation, DockableMeta, Layout, NodeId, NodeKind,
    ProportionalGroup, RootState, TabGroup, TabGroupKind,
};
pub use state::DockState;
pub use view::build_layout;
pub use widget::dock_surface::{DockMessage, DockSurface};
pub use widget::tab_dock::{tab_dock_simple, TabDock, TabMessage};
