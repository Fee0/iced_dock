//! Docking layout system for iced.
//!
//! ## Quick start
//!
//! ```ignore
//! use iced_dock::prelude::*;
//!
//! let session = DockSession::from_tree(layout_tree())?;
//! dock::<Message>()
//!     .state(session.state())
//!     .on_event(Message::DockEvent)
//!     .content(|key| view_panel(key))
//!     .build();
//! ```
//!
//! The dock widget applies layout mutations internally. Handle [`DockEvent`] in `update` for
//! side effects only — do not call [`DockSession::dispatch`] for widget-originated input.
//!
//! ## Serialization
//!
//! Enable the `serde` feature to derive `Serialize` and `Deserialize` on layout
//! types. Prefer declarative [`LayoutTree`] for workspace templates. Runtime [`Layout`] captures
//! split/tab state after user edits within the same application version; slotmap `NodeId` values
//! are not stable semantic handles across refactors.

pub mod builder;
pub mod error;
pub(crate) mod factory;
pub(crate) mod manager;
pub mod model;
pub mod prelude;
pub mod spatial;
pub mod style;
pub mod unstable;
pub mod widget;

pub use builder::{
    horizontal, panel, single, tabs, vertical, DockSession, InitialFocus, LayoutTree, PaneTarget,
    PanelCycle, PanelDef, SplitNode, TabsNode,
};
pub use error::{Error, Result};
pub use model::Layout;
pub use spatial::{adjacent_pane, pane_bounds_map, Direction};
pub use style::{
    close_button_style, constant, default, preset, Catalog, CloseButtonStyle, DockBackgroundStyle,
    DockStyle, DropOverlayStyle, PaneContent, SplitterStyle, StyleFn, TabBarStyle, TabStyle,
    WindowStyle,
};
pub use widget::{dock, Dock, DockAction, DockEvent, DockWidgetState, TabAction};
