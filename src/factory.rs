//! Layout tree mutations (Dock `IFactory` port).

use crate::model::{
    Axis, ContentKey, DockOperation, DockableMeta, Layout, NodeEntry, NodeId, NodeKind,
    ProportionalGroup, TabGroup, TabGroupKind,
};

/// Mutates a [`Layout`] tree.
#[derive(Debug, Default)]
pub struct Factory;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FactoryError {
    NodeNotFound,
    InvalidOperation,
    KindMismatch,
    EmptyTabGroup,
}

pub type FactoryResult<T> = Result<T, FactoryError>;

impl Factory {
    pub fn new() -> Self {
        Self
    }

    pub fn insert_document(
        &self,
        layout: &mut Layout,
        meta: DockableMeta,
    ) -> FactoryResult<NodeId> {
        let id = layout.nodes.insert(NodeEntry {
            kind: NodeKind::Document(meta),
            owner: None,
        });
        Ok(id)
    }

    pub fn insert_tool(&self, layout: &mut Layout, meta: DockableMeta) -> FactoryResult<NodeId> {
        let id = layout.nodes.insert(NodeEntry {
            kind: NodeKind::Tool(meta),
            owner: None,
        });
        Ok(id)
    }

    pub fn create_tab_group(
        &self,
        layout: &mut Layout,
        kind: TabGroupKind,
        owner: Option<NodeId>,
    ) -> FactoryResult<NodeId> {
        let id = layout.nodes.insert(NodeEntry {
            kind: NodeKind::TabGroup(TabGroup::new(kind)),
            owner,
        });
        Ok(id)
    }

    pub fn create_proportional(
        &self,
        layout: &mut Layout,
        axis: Axis,
        children: Vec<NodeId>,
        owner: Option<NodeId>,
    ) -> FactoryResult<NodeId> {
        let child_ids: Vec<NodeId> = children.clone();
        let id = layout.nodes.insert(NodeEntry {
            kind: NodeKind::Proportional(ProportionalGroup::new(axis, children)),
            owner,
        });
        for child in child_ids {
            layout.set_owner(child, Some(id));
        }
        Ok(id)
    }

    pub fn add_to_tab_group(
        &self,
        layout: &mut Layout,
        group: NodeId,
        leaf: NodeId,
    ) -> FactoryResult<()> {
        let leaf_kind = layout.leaf_kind(leaf).ok_or(FactoryError::KindMismatch)?;
        {
            let entry = layout.get_mut(group).ok_or(FactoryError::NodeNotFound)?;
            let NodeKind::TabGroup(ref mut tab_group) = entry.kind else {
                return Err(FactoryError::KindMismatch);
            };
            if tab_group.kind != leaf_kind {
                return Err(FactoryError::KindMismatch);
            }
            if !tab_group.children.contains(&leaf) {
                tab_group.children.push(leaf);
            }
            tab_group.active = Some(leaf);
        }
        layout.set_owner(leaf, Some(group));
        Ok(())
    }

    pub fn set_active(&self, layout: &mut Layout, group: NodeId, leaf: NodeId) -> FactoryResult<()> {
        let entry = layout.get_mut(group).ok_or(FactoryError::NodeNotFound)?;
        let NodeKind::TabGroup(ref mut tab_group) = entry.kind else {
            return Err(FactoryError::KindMismatch);
        };
        if tab_group.children.contains(&leaf) {
            tab_group.active = Some(leaf);
            Ok(())
        } else {
            Err(FactoryError::InvalidOperation)
        }
    }

    pub fn move_tab_to_group(
        &self,
        layout: &mut Layout,
        leaf: NodeId,
        target_group: NodeId,
        index: Option<usize>,
    ) -> FactoryResult<()> {
        self.remove_leaf_from_owner(layout, leaf)?;
        let leaf_kind = layout.leaf_kind(leaf).ok_or(FactoryError::KindMismatch)?;
        {
            let entry = layout.get_mut(target_group).ok_or(FactoryError::NodeNotFound)?;
            let NodeKind::TabGroup(ref mut tab_group) = entry.kind else {
                return Err(FactoryError::KindMismatch);
            };
            if tab_group.kind != leaf_kind {
                return Err(FactoryError::KindMismatch);
            }
            let idx = index.unwrap_or(tab_group.children.len());
            let idx = idx.min(tab_group.children.len());
            tab_group.children.insert(idx, leaf);
            tab_group.active = Some(leaf);
        }
        layout.set_owner(leaf, Some(target_group));
        self.collapse_empty_owners(layout, leaf)?;
        Ok(())
    }

    pub fn reorder_tab(
        &self,
        layout: &mut Layout,
        group: NodeId,
        from: usize,
        to: usize,
    ) -> FactoryResult<()> {
        let entry = layout.get_mut(group).ok_or(FactoryError::NodeNotFound)?;
        let NodeKind::TabGroup(ref mut tab_group) = entry.kind else {
            return Err(FactoryError::KindMismatch);
        };
        if from >= tab_group.children.len() || to >= tab_group.children.len() {
            return Err(FactoryError::InvalidOperation);
        }
        let id = tab_group.children.remove(from);
        tab_group.children.insert(to, id);
        Ok(())
    }

    pub fn close(&self, layout: &mut Layout, leaf: NodeId) -> FactoryResult<()> {
        let can_close = match layout.kind(leaf) {
            Some(NodeKind::Document(m) | NodeKind::Tool(m)) => m.can_close,
            _ => return Err(FactoryError::KindMismatch),
        };
        if !can_close {
            return Err(FactoryError::InvalidOperation);
        }
        let owner = layout.get(leaf).and_then(|e| e.owner);
        self.remove_leaf_from_owner(layout, leaf)?;
        layout.nodes.remove(leaf);
        if let Some(owner) = owner {
            self.collapse_empty_owners(layout, owner)?;
        }
        Ok(())
    }

    pub fn split(
        &self,
        layout: &mut Layout,
        target: NodeId,
        new_leaf: NodeId,
        operation: DockOperation,
    ) -> FactoryResult<()> {
        let axis = Axis::for_operation(operation).ok_or(FactoryError::InvalidOperation)?;
        let owner = layout.get(target).and_then(|e| e.owner);

        // If owner is proportional with same axis, insert into it.
        if let Some(owner_id) = owner {
            if let Some(NodeKind::Proportional(ref group)) = layout.kind(owner_id).cloned() {
                if group.axis == axis {
                    return self.insert_into_proportional(
                        layout,
                        owner_id,
                        target,
                        new_leaf,
                        operation,
                    );
                }
            }
        }

        let (first, second) = match operation {
            DockOperation::Left | DockOperation::Top => (new_leaf, target),
            DockOperation::Right | DockOperation::Bottom => (target, new_leaf),
            DockOperation::Fill => return Err(FactoryError::InvalidOperation),
        };

        let group = self.create_proportional(layout, axis, vec![first, second], owner)?;
        for &child in &[first, second] {
            layout.set_owner(child, Some(group));
        }

        self.replace_child_in_owner(layout, owner, target, group)?;
        Ok(())
    }

    pub fn dock_fill(
        &self,
        layout: &mut Layout,
        leaf: NodeId,
        target_group: NodeId,
    ) -> FactoryResult<()> {
        self.move_tab_to_group(layout, leaf, target_group, None)
    }

    pub fn set_proportions(
        &self,
        layout: &mut Layout,
        group: NodeId,
        proportions: Vec<f32>,
    ) -> FactoryResult<()> {
        let entry = layout.get_mut(group).ok_or(FactoryError::NodeNotFound)?;
        let NodeKind::Proportional(ref mut g) = entry.kind else {
            return Err(FactoryError::KindMismatch);
        };
        if proportions.len() != g.children.len() {
            return Err(FactoryError::InvalidOperation);
        }
        g.proportions = proportions;
        g.normalize_proportions();
        Ok(())
    }

    pub fn set_binary_split_ratio(
        &self,
        layout: &mut Layout,
        group: NodeId,
        split_at: f32,
    ) -> FactoryResult<()> {
        let entry = layout.get_mut(group).ok_or(FactoryError::NodeNotFound)?;
        let NodeKind::Proportional(ref mut g) = entry.kind else {
            return Err(FactoryError::KindMismatch);
        };
        if g.children.len() != 2 {
            return Err(FactoryError::InvalidOperation);
        }
        let split_at = split_at.clamp(0.05, 0.95);
        g.proportions = vec![split_at, 1.0 - split_at];
        Ok(())
    }

    /// Attach `child` as the sole root content (wrap in tab group if leaf).
    pub fn set_root_content(&self, layout: &mut Layout, child: NodeId) -> FactoryResult<()> {
        layout.set_root_child(Some(child));
        layout.set_owner(child, Some(layout.root));
        Ok(())
    }

    fn insert_into_proportional(
        &self,
        layout: &mut Layout,
        owner_id: NodeId,
        target: NodeId,
        new_leaf: NodeId,
        operation: DockOperation,
    ) -> FactoryResult<()> {
        let entry = layout.get_mut(owner_id).ok_or(FactoryError::NodeNotFound)?;
        let NodeKind::Proportional(ref mut group) = entry.kind else {
            return Err(FactoryError::KindMismatch);
        };
        let idx = group
            .children
            .iter()
            .position(|&c| c == target)
            .ok_or(FactoryError::NodeNotFound)?;

        let insert_idx = match operation {
            DockOperation::Left | DockOperation::Top => idx,
            DockOperation::Right | DockOperation::Bottom => idx + 1,
            _ => return Err(FactoryError::InvalidOperation),
        };

        group.children.insert(insert_idx, new_leaf);
        group.proportions.insert(insert_idx, 1.0);
        let n = group.children.len() as f32;
        for p in &mut group.proportions {
            *p = 1.0 / n;
        }
        layout.set_owner(new_leaf, Some(owner_id));
        Ok(())
    }

    fn replace_child_in_owner(
        &self,
        layout: &mut Layout,
        owner: Option<NodeId>,
        old_child: NodeId,
        new_child: NodeId,
    ) -> FactoryResult<()> {
        match owner {
            None => {
                layout.set_root_child(Some(new_child));
                layout.set_owner(new_child, Some(layout.root));
            }
            Some(owner_id) if owner_id == layout.root => {
                layout.set_root_child(Some(new_child));
                layout.set_owner(new_child, Some(layout.root));
            }
            Some(owner_id) => {
                let entry = layout.get_mut(owner_id).ok_or(FactoryError::NodeNotFound)?;
                match &mut entry.kind {
                    NodeKind::Proportional(ref mut g) => {
                        if let Some(pos) = g.children.iter().position(|&c| c == old_child) {
                            g.children[pos] = new_child;
                            layout.set_owner(new_child, Some(owner_id));
                        }
                    }
                    NodeKind::Root(ref mut r) => {
                        r.child = Some(new_child);
                        layout.set_owner(new_child, Some(owner_id));
                    }
                    _ => return Err(FactoryError::KindMismatch),
                }
            }
        }
        Ok(())
    }

    fn remove_leaf_from_owner(&self, layout: &mut Layout, leaf: NodeId) -> FactoryResult<()> {
        let owner = layout.get(leaf).and_then(|e| e.owner);
        let Some(owner_id) = owner else {
            return Ok(());
        };
        let entry = layout.get_mut(owner_id).ok_or(FactoryError::NodeNotFound)?;
        match &mut entry.kind {
            NodeKind::TabGroup(ref mut g) => {
                g.children.retain(|&c| c != leaf);
                if g.active == Some(leaf) {
                    g.active = g.children.last().copied();
                }
            }
            NodeKind::Proportional(ref mut g) => {
                if let Some(pos) = g.children.iter().position(|&c| c == leaf) {
                    g.children.remove(pos);
                    if pos < g.proportions.len() {
                        g.proportions.remove(pos);
                    }
                    if !g.proportions.is_empty() {
                        g.normalize_proportions();
                    }
                }
            }
            _ => {}
        }
        layout.set_owner(leaf, None);
        Ok(())
    }

    fn collapse_empty_owners(&self, layout: &mut Layout, start: NodeId) -> FactoryResult<()> {
        let mut current = Some(start);
        while let Some(id) = current {
            let (empty, owner, replacement) = {
                let entry = match layout.get(id) {
                    Some(e) => e,
                    None => break,
                };
                let owner = entry.owner;
                match &entry.kind {
                    NodeKind::TabGroup(g) if g.children.is_empty() => (true, owner, None),
                    NodeKind::Proportional(g) if g.children.len() == 1 => {
                        (true, owner, g.children.first().copied())
                    }
                    NodeKind::Proportional(g) if g.children.is_empty() => (true, owner, None),
                    _ => (false, owner, None),
                }
            };
            if !empty {
                break;
            }
            if let Some(repl) = replacement {
                self.replace_child_in_owner(layout, owner, id, repl)?;
                layout.nodes.remove(id);
                current = owner;
            } else {
                let own = owner;
                layout.nodes.remove(id);
                if let Some(o) = own {
                    self.collapse_empty_owners(layout, o)?;
                }
                break;
            }
        }
        Ok(())
    }
}

/// Helper to build a default IDE-like layout: root -> horizontal split -> doc tabs | tool tabs.
pub fn default_ide_layout(
    factory: &Factory,
    layout: &mut Layout,
    documents: Vec<(String, ContentKey)>,
    tools: Vec<(String, ContentKey)>,
) -> FactoryResult<()> {
    let mut doc_leaves = Vec::new();
    for (title, key) in documents {
        let meta = DockableMeta::new(format!("doc-{key:?}"), title, key);
        doc_leaves.push(factory.insert_document(layout, meta)?);
    }
    let mut tool_leaves = Vec::new();
    for (title, key) in tools {
        let meta = DockableMeta::new(format!("tool-{key:?}"), title, key);
        tool_leaves.push(factory.insert_tool(layout, meta)?);
    }

    let doc_group = factory.create_tab_group(layout, TabGroupKind::Document, None)?;
    for leaf in &doc_leaves {
        factory.add_to_tab_group(layout, doc_group, *leaf)?;
    }
    let tool_group = factory.create_tab_group(layout, TabGroupKind::Tool, None)?;
    for leaf in &tool_leaves {
        factory.add_to_tab_group(layout, tool_group, *leaf)?;
    }

    let root_split = factory.create_proportional(
        layout,
        Axis::Horizontal,
        vec![doc_group, tool_group],
        Some(layout.root),
    )?;
    layout.set_owner(doc_group, Some(root_split));
    layout.set_owner(tool_group, Some(root_split));
    factory.set_root_content(layout, root_split)?;
    Ok(())
}
