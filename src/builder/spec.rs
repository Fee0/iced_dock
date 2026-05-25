use std::collections::HashSet;

use crate::model::{Axis, ContentKey};
use crate::Error;

/// Declarative layout description.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum LayoutTree {
    /// Tabbed pane (typical split leaf).
    Tabs(TabsNode),
    /// Nested split container.
    Split(SplitNode),
}

/// Panel metadata used when building or opening tabs.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PanelDef {
    pub id: String,
    pub title: String,
    pub content: ContentKey,
    pub can_close: bool,
    pub can_drag: bool,
    pub can_drop: bool,
}

impl PanelDef {
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

    #[must_use]
    pub fn can_close(mut self, value: bool) -> Self {
        self.can_close = value;
        self
    }

    #[must_use]
    pub fn can_drag(mut self, value: bool) -> Self {
        self.can_drag = value;
        self
    }

    #[must_use]
    pub fn can_drop(mut self, value: bool) -> Self {
        self.can_drop = value;
        self
    }
}

/// Tabbed pane node.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TabsNode {
    pub name: Option<String>,
    pub panels: Vec<PanelDef>,
    pub active: Option<String>,
}

impl TabsNode {
    pub fn new(panels: impl IntoIterator<Item = PanelDef>) -> Self {
        Self {
            name: None,
            panels: panels.into_iter().collect(),
            active: None,
        }
    }

    #[must_use]
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    #[must_use]
    pub fn active(mut self, panel_id: impl Into<String>) -> Self {
        self.active = Some(panel_id.into());
        self
    }
}

/// Split container node.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SplitNode {
    pub axis: Axis,
    pub children: Vec<LayoutTree>,
    pub weights: Option<Vec<f32>>,
}

impl SplitNode {
    pub fn new(axis: Axis, children: impl IntoIterator<Item = LayoutTree>) -> Self {
        Self {
            axis,
            children: children.into_iter().collect(),
            weights: None,
        }
    }

    #[must_use]
    pub fn weights(mut self, weights: impl IntoIterator<Item = f32>) -> Self {
        self.weights = Some(weights.into_iter().collect());
        self
    }
}

impl LayoutTree {
    /// Set the active tab on a [`Tabs`] node.
    #[must_use]
    pub fn active(mut self, panel_id: impl Into<String>) -> Self {
        if let Self::Tabs(ref mut node) = self {
            node.active = Some(panel_id.into());
        }
        self
    }

    /// Assign a stable name to a [`Tabs`] node for [`PaneTarget::Named`](crate::builder::PaneTarget).
    #[must_use]
    pub fn named(mut self, name: impl Into<String>) -> Self {
        if let Self::Tabs(ref mut node) = self {
            node.name = Some(name.into());
        }
        self
    }

    /// Set split weights on a [`Split`] node.
    #[must_use]
    pub fn weights(mut self, weights: impl IntoIterator<Item = f32>) -> Self {
        if let Self::Split(ref mut node) = self {
            node.weights = Some(weights.into_iter().collect());
        }
        self
    }
}

/// Create a panel definition (for use inside [`tabs`]).
pub fn panel(id: impl Into<String>, title: impl Into<String>, content: ContentKey) -> PanelDef {
    PanelDef::new(id, title, content)
}

/// Create a tabbed pane node.
pub fn tabs(panels: impl IntoIterator<Item = PanelDef>) -> LayoutTree {
    LayoutTree::Tabs(TabsNode::new(panels))
}

/// Create a horizontal split.
pub fn horizontal(children: impl IntoIterator<Item = LayoutTree>) -> LayoutTree {
    LayoutTree::Split(SplitNode::new(Axis::Horizontal, children))
}

/// Create a vertical split.
pub fn vertical(children: impl IntoIterator<Item = LayoutTree>) -> LayoutTree {
    LayoutTree::Split(SplitNode::new(Axis::Vertical, children))
}

/// Single panel occupying the full dock area.
#[must_use]
pub fn single(def: PanelDef) -> LayoutTree {
    LayoutTree::Tabs(TabsNode::new([def]))
}

/// Validate a layout tree before compilation.
pub(crate) fn validate_tree(tree: &LayoutTree) -> crate::Result {
    let mut panel_ids = HashSet::new();
    let mut pane_names = HashSet::new();
    validate_node(tree, &mut panel_ids, &mut pane_names)
}

fn validate_node(
    tree: &LayoutTree,
    panel_ids: &mut HashSet<String>,
    pane_names: &mut HashSet<String>,
) -> crate::Result {
    match tree {
        LayoutTree::Tabs(node) => {
            if node.panels.is_empty() {
                return Err(Error::EmptyLayout);
            }
            if let Some(name) = &node.name {
                if !pane_names.insert(name.clone()) {
                    return Err(Error::DuplicatePaneName(name.clone()));
                }
            }
            let pane_label = node.name.clone().unwrap_or_else(|| "<unnamed>".into());
            for def in &node.panels {
                if !panel_ids.insert(def.id.clone()) {
                    return Err(Error::DuplicatePanelId(def.id.clone()));
                }
            }
            if let Some(active) = &node.active {
                if !node.panels.iter().any(|p| p.id == *active) {
                    return Err(Error::UnknownActivePanel {
                        pane_name: pane_label,
                        panel: active.clone(),
                    });
                }
            }
            Ok(())
        }
        LayoutTree::Split(node) => {
            if node.children.is_empty() {
                return Err(Error::EmptyLayout);
            }
            if let Some(weights) = &node.weights {
                if weights.len() != node.children.len() {
                    return Err(Error::InvalidWeights {
                        expected: node.children.len(),
                        got: weights.len(),
                    });
                }
            }
            for child in &node.children {
                validate_node(child, panel_ids, pane_names)?;
            }
            Ok(())
        }
    }
}
