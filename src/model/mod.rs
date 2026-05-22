//! Pure layout model (no iced types).

mod layout;
mod proportional;
mod tab;

pub use layout::{
    Axis, ContentKey, DockOperation, Layout, NodeEntry, NodeId, NodeKind, RootState,
};
pub use proportional::ProportionalGroup;
pub use tab::{DockableMeta, TabGroup, TabGroupKind};
