//! Declarative layout trees and [`DockSession`].

pub mod compile;
mod index;
mod session;
mod spec;

pub use index::DockIndex;
pub use session::{DockSession, InitialFocus, PaneTarget, PanelCycle};
pub use spec::{
    horizontal, panel, single, tabs, vertical, LayoutTree, PanelDef, SplitNode, TabsNode,
};
