use crate::model::{DockOperation, NodeId};

/// Crate-wide result alias.
pub type Result<T = ()> = std::result::Result<T, Error>;

/// Unified error type for the docking layout system.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Error {
    // --- Layout spec / builder ---
    #[error("layout tree is empty")]
    EmptyLayout,

    #[error("duplicate panel id: {0}")]
    DuplicatePanelId(String),

    #[error("duplicate pane name: {0}")]
    DuplicatePaneName(String),

    #[error("unknown active panel `{panel}` in pane `{pane_name}`")]
    UnknownActivePanel { pane_name: String, panel: String },

    #[error("expected {expected} weights, got {got}")]
    InvalidWeights { expected: usize, got: usize },

    #[error("unknown panel: {0}")]
    UnknownPanel(String),

    #[error("unknown pane: {0}")]
    UnknownPane(String),

    #[error("invalid pane target")]
    InvalidTarget,

    // --- Factory / layout mutations ---
    #[error("node {node:?} is not a panel leaf")]
    NotPanel { node: NodeId },

    #[error("node {node:?} is not a pane")]
    NotPane { node: NodeId },

    #[error("node {node:?} is not a proportional group")]
    NotProportional { node: NodeId },

    #[error("panel {panel:?} has no owner")]
    NoOwner { panel: NodeId },

    #[error("invalid split operation: {0:?}")]
    InvalidSplitOperation(DockOperation),

    #[error("operation requires an edge dock target")]
    NotEdgeOperation,

    #[error("invalid split target node {node:?}")]
    InvalidSplitTarget { node: NodeId },

    #[error("child {child:?} not found under parent {parent:?}")]
    ChildNotFound { parent: NodeId, child: NodeId },

    #[error("invalid parent {owner:?} for child {child:?}")]
    InvalidParent { owner: NodeId, child: NodeId },

    #[error("tab index out of bounds in pane {pane:?}: from={from}, to={to}, len={len}")]
    InvalidTabIndex {
        pane: NodeId,
        from: usize,
        to: usize,
        len: usize,
    },

    #[error("invalid splitter index {index} for group with {children} children")]
    InvalidSplitterIndex { index: usize, children: usize },

    #[error("proportional group weights sum to zero")]
    ZeroTotalWeight,

    // --- Drag / drop ---
    #[error("drag session has no hover target")]
    MissingHoverTarget,

    #[error("drag session has no dock operation")]
    MissingOperation,

    #[error("dock drop validation failed")]
    ValidationFailed,
}
