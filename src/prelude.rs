//! Common imports for application code.
//!
//! ```ignore
//! use iced_dock::prelude::*;
//! ```

pub use crate::builder::{
    horizontal, panel, single, tabs, vertical, DockSession, InitialFocus, LayoutTree, PaneTarget,
    PanelCycle, PanelDef,
};
pub use crate::model::ContentKey;
pub use crate::spatial::Direction;
pub use crate::style::{default, preset, Catalog, DockStyle, PaneContent, StyleFn};
pub use crate::widget::{dock, Dock, DockEvent, DockWidgetState};
pub use crate::{Error, Result};
