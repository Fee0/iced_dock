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
mod error;
mod index;
mod session;
mod spec;

pub use compile::{build_tree, BuiltLayout};
pub use error::LayoutError;
pub use index::DockIndex;
pub use session::{DockSession, PaneTarget};
pub use spec::{
    horizontal, panel, single, tabs, vertical, LayoutTree, PanelDef, SplitNode, TabsNode,
};
