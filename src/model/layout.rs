use slotmap::{new_key_type, SlotMap};

use super::pane::{Pane, Panel};
use super::ProportionalGroup;

new_key_type! {
    /// Stable node handle in the layout arena.
    pub struct NodeId;
}

/// Dock drop / split operations (floating window deferred).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DockOperation {
    Fill,
    Left,
    Right,
    Top,
    Bottom,
}

impl DockOperation {
    #[must_use]
    pub fn is_edge(self) -> bool {
        !matches!(self, Self::Fill)
    }
}

/// Split orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Axis {
    Horizontal,
    Vertical,
}

impl Axis {
    #[must_use]
    pub fn perpendicular(self) -> Self {
        match self {
            Self::Horizontal => Self::Vertical,
            Self::Vertical => Self::Horizontal,
        }
    }

    #[must_use]
    pub fn for_operation(op: DockOperation) -> Option<Self> {
        match op {
            DockOperation::Left | DockOperation::Right => Some(Self::Horizontal),
            DockOperation::Top | DockOperation::Bottom => Some(Self::Vertical),
            DockOperation::Fill => None,
        }
    }
}

/// Root wrapper (single child).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RootState {
    pub child: Option<NodeId>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum NodeKind<K> {
    Panel(Panel<K>),
    Pane(Pane),
    Proportional(ProportionalGroup),
    Root(RootState),
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NodeEntry<K> {
    pub kind: NodeKind<K>,
    pub owner: Option<NodeId>,
}

/// Layout tree arena.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Layout<K> {
    pub nodes: SlotMap<NodeId, NodeEntry<K>>,
    pub root: NodeId,
}

impl<K> Layout<K> {
    #[must_use]
    pub fn new() -> Self {
        let mut nodes = SlotMap::with_key();
        let root = nodes.insert(NodeEntry {
            kind: NodeKind::Root(RootState { child: None }),
            owner: None,
        });
        Self { nodes, root }
    }

    #[must_use]
    pub fn get(&self, id: NodeId) -> Option<&NodeEntry<K>> {
        self.nodes.get(id)
    }

    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut NodeEntry<K>> {
        self.nodes.get_mut(id)
    }

    #[must_use]
    pub fn kind(&self, id: NodeId) -> Option<&NodeKind<K>> {
        self.nodes.get(id).map(|e| &e.kind)
    }

    pub fn set_owner(&mut self, id: NodeId, owner: Option<NodeId>) {
        if let Some(entry) = self.nodes.get_mut(id) {
            entry.owner = owner;
        }
    }

    #[must_use]
    pub fn is_leaf(&self, id: NodeId) -> bool {
        matches!(self.kind(id), Some(NodeKind::Panel(_)))
    }

    #[must_use]
    pub fn root_child(&self) -> Option<NodeId> {
        match self.kind(self.root)? {
            NodeKind::Root(r) => r.child,
            _ => None,
        }
    }

    pub fn set_root_child(&mut self, child: Option<NodeId>) {
        if let Some(NodeKind::Root(ref mut r)) = self.nodes.get_mut(self.root).map(|e| &mut e.kind)
        {
            r.child = child;
        }
        if let Some(child) = child {
            self.set_owner(child, Some(self.root));
        }
    }
}

impl<K> Default for Layout<K> {
    fn default() -> Self {
        Self::new()
    }
}
