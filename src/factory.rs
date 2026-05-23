//! Tree mutations for the docking layout.

use crate::model::{
    Axis, ContentKey, DockOperation, DockableMeta, Layout, NodeEntry, NodeId, NodeKind,
    ProportionalGroup, TabGroup, TabGroupKind,
};

/// All structural changes to a [`Layout`].
#[derive(Debug, Clone, Copy, Default)]
pub struct Factory;

impl Factory {
    pub fn insert_document(
        &self,
        layout: &mut Layout,
        id: impl Into<String>,
        title: impl Into<String>,
        content: ContentKey,
    ) -> NodeId {
        layout.nodes.insert(NodeEntry {
            kind: NodeKind::Document(DockableMeta::new(id, title, content)),
            owner: None,
        })
    }

    pub fn insert_tool(
        &self,
        layout: &mut Layout,
        id: impl Into<String>,
        title: impl Into<String>,
        content: ContentKey,
    ) -> NodeId {
        layout.nodes.insert(NodeEntry {
            kind: NodeKind::Tool(DockableMeta::new(id, title, content)),
            owner: None,
        })
    }

    pub fn create_tab_group(&self, layout: &mut Layout, kind: TabGroupKind) -> NodeId {
        layout.nodes.insert(NodeEntry {
            kind: NodeKind::TabGroup(TabGroup::new(kind)),
            owner: None,
        })
    }

    pub fn create_proportional(
        &self,
        layout: &mut Layout,
        axis: Axis,
        children: Vec<NodeId>,
    ) -> NodeId {
        let id = layout.nodes.insert(NodeEntry {
            kind: NodeKind::Proportional(ProportionalGroup::new(axis, children.clone())),
            owner: None,
        });
        for child in children {
            layout.set_owner(child, Some(id));
        }
        id
    }

    pub fn add_to_tab_group(
        &self,
        layout: &mut Layout,
        group: NodeId,
        leaf: NodeId,
    ) -> Result<(), ()> {
        let kind = layout.tab_group_kind(group).ok_or(())?;
        let leaf_kind = layout.leaf_kind(leaf).ok_or(())?;
        if kind != leaf_kind {
            return Err(());
        }
        if let Some(NodeKind::TabGroup(ref mut g)) = layout.get_mut(group).map(|e| &mut e.kind) {
            g.children.push(leaf);
            g.active = Some(leaf);
            layout.set_owner(leaf, Some(group));
            Ok(())
        } else {
            Err(())
        }
    }

    pub fn move_tab_to_group(
        &self,
        layout: &mut Layout,
        source: NodeId,
        target_group: NodeId,
    ) -> Result<(), ()> {
        self.remove_from_parent(layout, source)?;
        self.add_to_tab_group(layout, target_group, source)
    }

    pub fn dock_fill(
        &self,
        layout: &mut Layout,
        source: NodeId,
        target_group: NodeId,
    ) -> Result<(), ()> {
        self.move_tab_to_group(layout, source, target_group)
    }

    pub fn reorder_tab(
        &self,
        layout: &mut Layout,
        group: NodeId,
        from: usize,
        to: usize,
    ) -> Result<(), ()> {
        if let Some(NodeKind::TabGroup(ref mut g)) = layout.get_mut(group).map(|e| &mut e.kind) {
            if from < g.children.len() && to < g.children.len() {
                let child = g.children.remove(from);
                g.children.insert(to, child);
                Ok(())
            } else {
                Err(())
            }
        } else {
            Err(())
        }
    }

    pub fn set_active_tab(&self, layout: &mut Layout, group: NodeId, leaf: NodeId) {
        if let Some(NodeKind::TabGroup(ref mut g)) = layout.get_mut(group).map(|e| &mut e.kind) {
            if g.children.contains(&leaf) {
                g.active = Some(leaf);
            }
        }
    }

    pub fn close(&self, layout: &mut Layout, leaf: NodeId) -> Result<(), ()> {
        let owner = layout.get(leaf).and_then(|e| e.owner);
        let owner = owner.ok_or(())?;
        self.remove_from_parent(layout, leaf)?;
        layout.nodes.remove(leaf);
        self.collapse_owner(layout, owner);
        Ok(())
    }

    pub fn split(
        &self,
        layout: &mut Layout,
        source: NodeId,
        target: NodeId,
        op: DockOperation,
    ) -> Result<(), ()> {
        if op == DockOperation::Fill {
            return Err(());
        }
        let axis = Axis::for_operation(op).ok_or(())?;

        // Remove source from its current parent first
        let old_owner = layout.get(source).and_then(|e| e.owner);
        self.remove_from_parent(layout, source)?;
        if let Some(owner) = old_owner {
            self.collapse_owner(layout, owner);
        }

        let split_target = self.resolve_split_target(layout, target, op)?;
        let new_leaf_side = self.side_for_operation(op);

        if let Some(parent) = layout.get(split_target).and_then(|e| e.owner) {
            if let Some(NodeKind::Proportional(ref pg)) = layout.kind(parent) {
                if pg.axis == axis {
                    return self.insert_into_proportional(
                        layout,
                        parent,
                        split_target,
                        source,
                        new_leaf_side,
                    );
                }
            }
        }

        // Wrap target in new two-child proportional
        let old_owner = layout.get(split_target).and_then(|e| e.owner);
        let prop = self.create_proportional(
            layout,
            axis,
            if new_leaf_side {
                vec![split_target, source]
            } else {
                vec![source, split_target]
            },
        );

        if let Some(owner) = old_owner {
            self.replace_child(layout, owner, split_target, prop)?;
            layout.set_owner(prop, Some(owner));
        } else {
            layout.set_root_child(Some(prop));
        }

        layout.set_owner(source, Some(prop));
        layout.set_owner(split_target, Some(prop));
        Ok(())
    }

    fn resolve_split_target(
        &self,
        layout: &Layout,
        target: NodeId,
        _op: DockOperation,
    ) -> Result<NodeId, ()> {
        match layout.kind(target) {
            Some(NodeKind::TabGroup(_))
            | Some(NodeKind::Proportional(_))
            | Some(NodeKind::Document(_))
            | Some(NodeKind::Tool(_)) => Ok(target),
            _ => Err(()),
        }
    }

    fn side_for_operation(&self, op: DockOperation) -> bool {
        // true = new pane is second (right/bottom)
        matches!(
            op,
            DockOperation::Right | DockOperation::Bottom
        )
    }

    fn insert_into_proportional(
        &self,
        layout: &mut Layout,
        parent: NodeId,
        target: NodeId,
        source: NodeId,
        after: bool,
    ) -> Result<(), ()> {
        if let Some(NodeKind::Proportional(ref mut pg)) = layout.get_mut(parent).map(|e| &mut e.kind)
        {
            if let Some(idx) = pg.children.iter().position(|&c| c == target) {
                let insert_at = if after { idx + 1 } else { idx };
                pg.children.insert(insert_at, source);
                pg.proportions.insert(insert_at, 1.0);
                pg.normalize_proportions();
                layout.set_owner(source, Some(parent));
                return Ok(());
            }
        }
        Err(())
    }

    fn replace_child(
        &self,
        layout: &mut Layout,
        parent: NodeId,
        old_child: NodeId,
        new_child: NodeId,
    ) -> Result<(), ()> {
        match layout.kind(parent) {
            Some(NodeKind::Root(_)) => {
                layout.set_root_child(Some(new_child));
                Ok(())
            }
            Some(NodeKind::Proportional(_)) => {
                if let Some(NodeKind::Proportional(ref mut pg)) =
                    layout.get_mut(parent).map(|e| &mut e.kind)
                {
                    if let Some(idx) = pg.children.iter().position(|&c| c == old_child) {
                        pg.children[idx] = new_child;
                        layout.set_owner(new_child, Some(parent));
                        return Ok(());
                    }
                }
                Err(())
            }
            Some(NodeKind::TabGroup(_)) => {
                if let Some(NodeKind::TabGroup(ref mut g)) =
                    layout.get_mut(parent).map(|e| &mut e.kind)
                {
                    for c in &mut g.children {
                        if *c == old_child {
                            *c = new_child;
                            if g.active == Some(old_child) {
                                g.active = Some(new_child);
                            }
                            layout.set_owner(new_child, Some(parent));
                            return Ok(());
                        }
                    }
                }
                Err(())
            }
            _ => Err(()),
        }
    }

    pub fn remove_from_parent(&self, layout: &mut Layout, child: NodeId) -> Result<(), ()> {
        let owner = layout.get(child).and_then(|e| e.owner).ok_or(())?;
        match layout.kind(owner) {
            Some(NodeKind::TabGroup(_)) => {
                if let Some(NodeKind::TabGroup(ref mut g)) =
                    layout.get_mut(owner).map(|e| &mut e.kind)
                {
                    g.children.retain(|&c| c != child);
                    if g.active == Some(child) {
                        g.active = g.children.last().copied();
                    }
                }
                layout.set_owner(child, None);
                Ok(())
            }
            Some(NodeKind::Proportional(_)) => {
                if let Some(NodeKind::Proportional(ref mut pg)) =
                    layout.get_mut(owner).map(|e| &mut e.kind)
                {
                    if let Some(idx) = pg.children.iter().position(|&c| c == child) {
                        pg.children.remove(idx);
                        if idx < pg.proportions.len() {
                            pg.proportions.remove(idx);
                        }
                        pg.normalize_proportions();
                    }
                }
                layout.set_owner(child, None);
                Ok(())
            }
            _ => Err(()),
        }
    }

    fn collapse_owner(&self, layout: &mut Layout, owner: NodeId) {
        if owner == layout.root {
            let empty_child = match layout.kind(layout.root) {
                Some(NodeKind::Root(r)) => r.child.and_then(|child| {
                    layout
                        .get(child)
                        .map(|e| matches!(e.kind, NodeKind::TabGroup(ref g) if g.children.is_empty()))
                }),
                _ => None,
            };
            if empty_child == Some(true) {
                if let Some(NodeKind::Root(ref mut r)) =
                    layout.get_mut(owner).map(|e| &mut e.kind)
                {
                    r.child = None;
                }
            }
            return;
        }

        let should_collapse = match layout.kind(owner) {
            Some(NodeKind::TabGroup(g)) => g.children.is_empty(),
            Some(NodeKind::Proportional(pg)) => pg.children.len() <= 1,
            _ => false,
        };

        if !should_collapse {
            return;
        }

        let grand_owner = layout.get(owner).and_then(|e| e.owner);
        let replacement = match layout.kind(owner) {
            Some(NodeKind::Proportional(pg)) => pg.children.first().copied(),
            _ => None,
        };

        if let Some(go) = grand_owner {
            if let Some(rep) = replacement {
                let _ = self.replace_child(layout, go, owner, rep);
                layout.nodes.remove(owner);
                self.collapse_owner(layout, go);
            } else if go == layout.root {
                layout.set_root_child(None);
                layout.nodes.remove(owner);
            }
        }
    }

    pub fn set_proportions(
        &self,
        layout: &mut Layout,
        group: NodeId,
        proportions: Vec<f32>,
    ) -> Result<(), ()> {
        if let Some(NodeKind::Proportional(ref mut pg)) = layout.get_mut(group).map(|e| &mut e.kind)
        {
            if proportions.len() == pg.children.len() {
                pg.proportions = proportions;
                pg.normalize_proportions();
                Ok(())
            } else {
                Err(())
            }
        } else {
            Err(())
        }
    }

    pub fn set_binary_split_ratio(
        &self,
        layout: &mut Layout,
        group: NodeId,
        split_at: f32,
    ) -> Result<(), ()> {
        let ratio = split_at.clamp(0.1, 0.9);
        self.set_proportions(layout, group, vec![ratio, 1.0 - ratio])
    }

    pub fn adjust_splitter(
        &self,
        layout: &mut Layout,
        group: NodeId,
        splitter_index: usize,
        ratio_before: f32,
    ) -> Result<(), ()> {
        if let Some(NodeKind::Proportional(ref mut pg)) = layout.get_mut(group).map(|e| &mut e.kind)
        {
            let n = pg.children.len();
            if splitter_index >= n.saturating_sub(1) {
                return Err(());
            }
            let mut props = if pg.proportions.len() == n {
                pg.proportions.clone()
            } else {
                vec![1.0; n]
            };
            let left_sum: f32 = props[..=splitter_index].iter().sum();
            let right_sum: f32 = props[splitter_index + 1..].iter().sum();
            let total = left_sum + right_sum;
            if total <= 0.0 {
                return Err(());
            }
            let new_left = ratio_before * total;
            let new_right = total - new_left;
            if splitter_index == 0 {
                props[0] = new_left;
                for p in &mut props[1..] {
                    *p = new_right / (n - 1) as f32;
                }
            } else {
                let per_left = new_left / (splitter_index + 1) as f32;
                for p in props[..=splitter_index].iter_mut() {
                    *p = per_left;
                }
                let per_right = new_right / (n - splitter_index - 1) as f32;
                for p in props[splitter_index + 1..].iter_mut() {
                    *p = per_right;
                }
            }
            pg.proportions = props;
            pg.normalize_proportions();
            Ok(())
        } else {
            Err(())
        }
    }

    /// IDE-style 5–6 pane bootstrap layout (fixed demo content keys).
    pub fn complex_ide_layout(&self, layout: &mut Layout) -> Result<(), ()> {
        let d_main = self.insert_document(layout, "main", "main.rs", ContentKey(0));
        let d_lib = self.insert_document(layout, "lib", "lib.rs", ContentKey(1));
        let doc_group1 = self.create_tab_group(layout, TabGroupKind::Document);
        self.add_to_tab_group(layout, doc_group1, d_main)?;
        self.add_to_tab_group(layout, doc_group1, d_lib)?;

        let d_prev = self.insert_document(layout, "preview", "preview", ContentKey(2));
        let doc_group2 = self.create_tab_group(layout, TabGroupKind::Document);
        self.add_to_tab_group(layout, doc_group2, d_prev)?;

        let left_col = self.create_proportional(
            layout,
            Axis::Vertical,
            vec![doc_group1, doc_group2],
        );
        if let Some(NodeKind::Proportional(ref mut pg)) =
            layout.get_mut(left_col).map(|e| &mut e.kind)
        {
            pg.proportions = vec![0.55, 0.45];
        }

        let t_prop = self.insert_tool(layout, "props", "Properties", ContentKey(10));
        let t_out = self.insert_tool(layout, "output", "Output", ContentKey(11));
        let tool_group1 = self.create_tab_group(layout, TabGroupKind::Tool);
        self.add_to_tab_group(layout, tool_group1, t_prop)?;
        self.add_to_tab_group(layout, tool_group1, t_out)?;

        let t_exp = self.insert_tool(layout, "explorer", "Explorer", ContentKey(12));
        let t_srch = self.insert_tool(layout, "search", "Search", ContentKey(13));
        let tool_group2 = self.create_tab_group(layout, TabGroupKind::Tool);
        self.add_to_tab_group(layout, tool_group2, t_exp)?;
        self.add_to_tab_group(layout, tool_group2, t_srch)?;

        let right_col = self.create_proportional(
            layout,
            Axis::Vertical,
            vec![tool_group1, tool_group2],
        );
        if let Some(NodeKind::Proportional(ref mut pg)) =
            layout.get_mut(right_col).map(|e| &mut e.kind)
        {
            pg.proportions = vec![0.5, 0.5];
        }

        let main = self.create_proportional(layout, Axis::Horizontal, vec![left_col, right_col]);
        if let Some(NodeKind::Proportional(ref mut pg)) = layout.get_mut(main).map(|e| &mut e.kind) {
            pg.proportions = vec![0.72, 0.28];
        }

        layout.set_root_child(Some(main));
        Ok(())
    }
}
