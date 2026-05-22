use super::{ContentKey, NodeId};

/// Document vs tool tab hosts (no cross-mixing in MVP).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabGroupKind {
    Document,
    Tool,
}

/// Tabbed host for document or tool leaves.
#[derive(Debug, Clone)]
pub struct TabGroup {
    pub kind: TabGroupKind,
    pub children: Vec<NodeId>,
    pub active: Option<NodeId>,
}

impl TabGroup {
    pub fn new(kind: TabGroupKind) -> Self {
        Self {
            kind,
            children: Vec::new(),
            active: None,
        }
    }

    pub fn active_index(&self) -> Option<usize> {
        let active = self.active?;
        self.children.iter().position(|&id| id == active)
    }
}

/// Metadata stored on leaf nodes.
#[derive(Debug, Clone)]
pub struct DockableMeta {
    pub id: String,
    pub title: String,
    pub content: ContentKey,
    pub can_close: bool,
    pub can_drag: bool,
    pub can_drop: bool,
}

impl DockableMeta {
    pub fn new(id: impl Into<String>, title: impl Into<String>, content: ContentKey) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            content,
            can_close: true,
            can_drag: true,
            can_drop: true,
        }
    }
}
