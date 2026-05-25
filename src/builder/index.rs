use std::collections::HashMap;

use crate::model::{Layout, NodeId, NodeKind};

/// String-id lookups for panels and named panes.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DockIndex {
    pub panels: HashMap<String, NodeId>,
    pub panes: HashMap<String, NodeId>,
}

impl DockIndex {
    #[must_use]
    pub fn rebuild_from_layout(layout: &Layout) -> Self {
        let mut index = Self::default();
        for (id, entry) in &layout.nodes {
            match &entry.kind {
                NodeKind::Panel(panel) => {
                    index.panels.insert(panel.id.clone(), id);
                }
                NodeKind::Pane(pane) => {
                    if let Some(name) = &pane.name {
                        index.panes.insert(name.clone(), id);
                    }
                }
                _ => {}
            }
        }
        index
    }

    pub fn panel_ids(&self) -> impl Iterator<Item = &String> {
        self.panels.keys()
    }

    #[must_use]
    pub fn pane_node(&self, name: &str) -> Option<NodeId> {
        self.panes.get(name).copied()
    }

    #[must_use]
    pub fn panel_node(&self, id: &str) -> Option<NodeId> {
        self.panels.get(id).copied()
    }
}
