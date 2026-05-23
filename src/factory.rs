//! Tree mutations for the docking layout.

use crate::model::{
    Axis, ContentKey, DockOperation, Layout, NodeEntry, NodeId, NodeKind, Panel, Pane,
    ProportionalGroup,
};

/// All structural changes to a [`Layout`].
#[derive(Debug, Clone, Copy, Default)]
pub struct Factory;

impl Factory {
    pub fn insert_panel(
        &self,
        layout: &mut Layout,
        id: impl Into<String>,
        title: impl Into<String>,
        content: ContentKey,
    ) -> NodeId {
        layout.nodes.insert(NodeEntry {
            kind: NodeKind::Panel(Panel::new(id, title, content)),
            owner: None,
        })
    }

    pub fn create_pane(&self, layout: &mut Layout) -> NodeId {
        layout.nodes.insert(NodeEntry {
            kind: NodeKind::Pane(Pane::new()),
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

    pub fn add_panel_to_pane(
        &self,
        layout: &mut Layout,
        pane: NodeId,
        panel: NodeId,
    ) -> Result<(), ()> {
        if !layout.is_leaf(panel) {
            return Err(());
        }
        if let Some(NodeKind::Pane(ref mut p)) = layout.get_mut(pane).map(|e| &mut e.kind) {
            p.tabs.push(panel);
            p.active = Some(panel);
            layout.set_owner(panel, Some(pane));
            Ok(())
        } else {
            Err(())
        }
    }

    pub fn move_panel_to_pane(
        &self,
        layout: &mut Layout,
        source: NodeId,
        target_pane: NodeId,
    ) -> Result<(), ()> {
        self.remove_from_parent(layout, source)?;
        self.add_panel_to_pane(layout, target_pane, source)
    }

    pub fn dock_fill(
        &self,
        layout: &mut Layout,
        source_panel: NodeId,
        target_pane: NodeId,
    ) -> Result<(), ()> {
        let old_owner = layout.get(source_panel).and_then(|e| e.owner);
        self.move_panel_to_pane(layout, source_panel, target_pane)?;
        if let Some(owner) = old_owner {
            self.collapse_owner(layout, owner);
        }
        Ok(())
    }

    pub fn reorder_panel(
        &self,
        layout: &mut Layout,
        pane: NodeId,
        from: usize,
        to: usize,
    ) -> Result<(), ()> {
        if let Some(NodeKind::Pane(ref mut p)) = layout.get_mut(pane).map(|e| &mut e.kind) {
            if from < p.tabs.len() && to < p.tabs.len() {
                let child = p.tabs.remove(from);
                p.tabs.insert(to, child);
                Ok(())
            } else {
                Err(())
            }
        } else {
            Err(())
        }
    }

    pub fn set_active_panel(&self, layout: &mut Layout, pane: NodeId, panel: NodeId) {
        if let Some(NodeKind::Pane(ref mut p)) = layout.get_mut(pane).map(|e| &mut e.kind) {
            if p.tabs.contains(&panel) {
                p.active = Some(panel);
            }
        }
    }

    pub fn close(&self, layout: &mut Layout, panel: NodeId) -> Result<(), ()> {
        let owner = layout.get(panel).and_then(|e| e.owner);
        let owner = owner.ok_or(())?;
        self.remove_from_parent(layout, panel)?;
        layout.nodes.remove(panel);
        self.collapse_owner(layout, owner);
        Ok(())
    }

    /// Split `target` and place `source` pane beside it.
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

        if let Some(old_owner) = layout.get(source).and_then(|e| e.owner) {
            self.remove_from_parent(layout, source)?;
            self.collapse_owner(layout, old_owner);
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

    /// Edge drop on the same pane as the drag source.
    pub fn split_same_pane_edge(
        &self,
        layout: &mut Layout,
        pane: NodeId,
        panel: NodeId,
        op: DockOperation,
    ) -> Result<(), ()> {
        if !op.is_edge() {
            return Err(());
        }
        let tab_count = match layout.kind(pane) {
            Some(NodeKind::Pane(p)) => p.tabs.len(),
            _ => return Err(()),
        };

        if tab_count > 1 {
            let new_pane = self.peel_panel_to_new_pane(layout, panel)?;
            self.split(layout, new_pane, pane, op)
        } else {
            let axis = Axis::for_operation(op).ok_or(())?;
            let after = self.side_for_operation(op);
            let empty_pane = self.create_pane(layout);
            let old_owner = layout.get(pane).and_then(|e| e.owner);

            let children = if after {
                vec![pane, empty_pane]
            } else {
                vec![empty_pane, pane]
            };
            let prop = self.create_proportional(layout, axis, children);

            if let Some(owner) = old_owner {
                self.replace_child(layout, owner, pane, prop)?;
                layout.set_owner(prop, Some(owner));
            } else {
                layout.set_root_child(Some(prop));
            }
            layout.set_owner(pane, Some(prop));
            layout.set_owner(empty_pane, Some(prop));
            Ok(())
        }
    }

    /// Edge drop onto a different pane than the drag source.
    pub fn split_cross_pane_edge(
        &self,
        layout: &mut Layout,
        source_pane: NodeId,
        source_panel: NodeId,
        target: NodeId,
        op: DockOperation,
    ) -> Result<(), ()> {
        if !op.is_edge() {
            return Err(());
        }
        let tab_count = match layout.kind(source_pane) {
            Some(NodeKind::Pane(p)) => p.tabs.len(),
            _ => return Err(()),
        };

        if tab_count > 1 {
            let new_pane = self.peel_panel_to_new_pane(layout, source_panel)?;
            self.split(layout, new_pane, target, op)
        } else {
            self.split(layout, source_pane, target, op)
        }
    }

    fn peel_panel_to_new_pane(&self, layout: &mut Layout, panel: NodeId) -> Result<NodeId, ()> {
        let new_pane = self.create_pane(layout);
        self.remove_from_parent(layout, panel)?;
        self.add_panel_to_pane(layout, new_pane, panel)?;
        Ok(new_pane)
    }

    fn resolve_split_target(
        &self,
        layout: &Layout,
        target: NodeId,
        _op: DockOperation,
    ) -> Result<NodeId, ()> {
        match layout.kind(target) {
            Some(NodeKind::Pane(_))
            | Some(NodeKind::Proportional(_))
            | Some(NodeKind::Panel(_)) => Ok(target),
            _ => Err(()),
        }
    }

    fn side_for_operation(&self, op: DockOperation) -> bool {
        matches!(op, DockOperation::Right | DockOperation::Bottom)
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
            Some(NodeKind::Pane(_)) => {
                if let Some(NodeKind::Pane(ref mut p)) = layout.get_mut(parent).map(|e| &mut e.kind)
                {
                    for t in &mut p.tabs {
                        if *t == old_child {
                            *t = new_child;
                            if p.active == Some(old_child) {
                                p.active = Some(new_child);
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
            Some(NodeKind::Pane(_)) => {
                if let Some(NodeKind::Pane(ref mut p)) = layout.get_mut(owner).map(|e| &mut e.kind)
                {
                    p.tabs.retain(|&c| c != child);
                    if p.active == Some(child) {
                        p.active = p.tabs.last().copied();
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
                    layout.get(child).map(|e| {
                        matches!(e.kind, NodeKind::Pane(ref p) if p.tabs.is_empty())
                    })
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
            Some(NodeKind::Pane(p)) => p.tabs.is_empty(),
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
            } else {
                let _ = self.remove_from_parent(layout, owner);
                layout.nodes.remove(owner);
                self.collapse_owner(layout, go);
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
            let total: f32 = props.iter().sum();
            if total <= 0.0 {
                return Err(());
            }
            let left_fixed: f32 = props[..splitter_index].iter().sum();
            let pair_total = props[splitter_index] + props[splitter_index + 1];
            let min_weight = 0.05 * total;
            let left = (ratio_before * total - left_fixed)
                .clamp(min_weight, pair_total - min_weight);
            props[splitter_index] = left;
            props[splitter_index + 1] = pair_total - left;
            pg.proportions = props;
            pg.normalize_proportions();
            Ok(())
        } else {
            Err(())
        }
    }

    /// IDE-style bootstrap layout (fixed demo content keys).
    pub fn complex_ide_layout(&self, layout: &mut Layout) -> Result<(), ()> {
        let p_main = self.insert_panel(layout, "main", "main.rs", ContentKey(0));
        let p_lib = self.insert_panel(layout, "lib", "lib.rs", ContentKey(1));
        let pane_left_top = self.create_pane(layout);
        self.add_panel_to_pane(layout, pane_left_top, p_main)?;
        self.add_panel_to_pane(layout, pane_left_top, p_lib)?;

        let p_prev = self.insert_panel(layout, "preview", "preview", ContentKey(2));
        let pane_left_bot = self.create_pane(layout);
        self.add_panel_to_pane(layout, pane_left_bot, p_prev)?;

        let left_col = self.create_proportional(
            layout,
            Axis::Vertical,
            vec![pane_left_top, pane_left_bot],
        );
        if let Some(NodeKind::Proportional(ref mut pg)) =
            layout.get_mut(left_col).map(|e| &mut e.kind)
        {
            pg.proportions = vec![0.55, 0.45];
        }

        let p_prop = self.insert_panel(layout, "props", "Properties", ContentKey(10));
        let p_out = self.insert_panel(layout, "output", "Output", ContentKey(11));
        let pane_right_top = self.create_pane(layout);
        self.add_panel_to_pane(layout, pane_right_top, p_prop)?;
        self.add_panel_to_pane(layout, pane_right_top, p_out)?;

        let p_exp = self.insert_panel(layout, "explorer", "Explorer", ContentKey(12));
        let p_srch = self.insert_panel(layout, "search", "Search", ContentKey(13));
        let pane_right_bot = self.create_pane(layout);
        self.add_panel_to_pane(layout, pane_right_bot, p_exp)?;
        self.add_panel_to_pane(layout, pane_right_bot, p_srch)?;

        let right_col = self.create_proportional(
            layout,
            Axis::Vertical,
            vec![pane_right_top, pane_right_bot],
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
