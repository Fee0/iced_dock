use slotmap::{new_key_type, SlotMap};

use super::pane::{Panel, Pane};
use super::ProportionalGroup;

new_key_type! {
    /// Stable node handle in the layout arena.
    pub struct NodeId;
}

/// Application content identity (documents/tools map UI in the host app).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContentKey(pub u32);

/// Dock drop / split operations (floating window deferred).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockOperation {
    Fill,
    Left,
    Right,
    Top,
    Bottom,
}

impl DockOperation {
    pub fn is_edge(self) -> bool {
        !matches!(self, Self::Fill)
    }
}

/// Split orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    Horizontal,
    Vertical,
}

impl Axis {
    pub fn perpendicular(self) -> Self {
        match self {
            Self::Horizontal => Self::Vertical,
            Self::Vertical => Self::Horizontal,
        }
    }

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
pub struct RootState {
    pub child: Option<NodeId>,
}

#[derive(Debug, Clone)]
pub enum NodeKind {
    Panel(Panel),
    Pane(Pane),
    Proportional(ProportionalGroup),
    Root(RootState),
}

#[derive(Debug, Clone)]
pub struct NodeEntry {
    pub kind: NodeKind,
    pub owner: Option<NodeId>,
}

/// Layout tree arena.
#[derive(Debug, Clone)]
pub struct Layout {
    pub nodes: SlotMap<NodeId, NodeEntry>,
    pub root: NodeId,
}

impl Layout {
    pub fn new() -> Self {
        let mut nodes = SlotMap::with_key();
        let root = nodes.insert(NodeEntry {
            kind: NodeKind::Root(RootState { child: None }),
            owner: None,
        });
        Self { nodes, root }
    }

    pub fn get(&self, id: NodeId) -> Option<&NodeEntry> {
        self.nodes.get(id)
    }

    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut NodeEntry> {
        self.nodes.get_mut(id)
    }

    pub fn kind(&self, id: NodeId) -> Option<&NodeKind> {
        self.nodes.get(id).map(|e| &e.kind)
    }

    pub fn set_owner(&mut self, id: NodeId, owner: Option<NodeId>) {
        if let Some(entry) = self.nodes.get_mut(id) {
            entry.owner = owner;
        }
    }

    pub fn is_leaf(&self, id: NodeId) -> bool {
        matches!(self.kind(id), Some(NodeKind::Panel(_)))
    }

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

impl Default for Layout {
    fn default() -> Self {
        Self::new()
    }
}
