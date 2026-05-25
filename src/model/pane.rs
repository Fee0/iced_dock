use super::{ContentKey, NodeId};

/// Single tab content (leaf node).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Panel {
    pub id: String,
    pub title: String,
    pub content: ContentKey,
    pub can_close: bool,
    pub can_drag: bool,
    pub can_drop: bool,
}

impl Panel {
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

/// Tabbed pane host.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Pane {
    pub name: Option<String>,
    pub tabs: Vec<NodeId>,
    pub active: Option<NodeId>,
}

impl Default for Pane {
    fn default() -> Self {
        Self::new()
    }
}

impl Pane {
    pub fn new() -> Self {
        Self {
            name: None,
            tabs: Vec::new(),
            active: None,
        }
    }

    pub fn active_index(&self) -> Option<usize> {
        let active = self.active?;
        self.tabs.iter().position(|&id| id == active)
    }
}
