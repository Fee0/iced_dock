//! High-level declarative API for building and managing dock layouts.
//!
//! # Before (manual graph construction)
//!
//! ```ignore
//! let factory = Factory;
//! let mut layout = Layout::new();
//! let panel = factory.insert_panel(&mut layout, "main", "main.rs", ContentKey(0));
//! let pane = factory.create_pane(&mut layout);
//! factory.add_panel_to_pane(&mut layout, pane, panel)?;
//! layout.set_root_child(Some(pane));
//! ```
//!
//! # After (declarative tree + session)
//!
//! ```ignore
//! use iced_dock::{DockSession, LayoutTree, panel, tabs, horizontal};
//!
//! let session = DockSession::from_tree(tabs([panel("main", "main.rs", ContentKey(0))]))?;
//! dock::<Message>().state(session.state()).build();
//! session.open_panel(PaneTarget::First, panel("doc2", "doc2.rs", ContentKey(1)))?;
//! ```

mod compile;
mod index;
mod session;
mod spec;

pub use compile::{
    active_panel_in_pane, build_tree, first_pane, owning_pane, pane_for_panel, BuiltLayout,
};
pub use index::DockIndex;
pub use session::{DockSession, InitialFocus, PanelCycle, PaneTarget};
pub use spec::{
    horizontal, panel, single, tabs, vertical, LayoutTree, PanelDef, SplitNode, TabsNode,
};
