use super::NodeId;

/// Single tab content (leaf node).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Panel<K> {
    pub id: String,
    pub title: String,
    pub content: K,
    pub can_close: bool,
    pub can_drag: bool,
    pub can_drop: bool,
    pub group: Option<String>,
}

impl<K: Copy> Panel<K> {
    pub fn new(id: impl Into<String>, title: impl Into<String>, content: K) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            content,
            can_close: true,
            can_drag: true,
            can_drop: true,
            group: None,
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
    pub group: Option<String>,
    pub persistent: bool,
}

impl Default for Pane {
    fn default() -> Self {
        Self::new()
    }
}

impl Pane {
    #[must_use]
    pub fn new() -> Self {
        Self {
            name: None,
            tabs: Vec::new(),
            active: None,
            group: None,
            persistent: false,
        }
    }

    #[must_use]
    pub fn active_index(&self) -> Option<usize> {
        let active = self.active?;
        self.tabs.iter().position(|&id| id == active)
    }
}
