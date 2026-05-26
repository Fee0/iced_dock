//! Drag validation and drop execution.

use iced::Rectangle;

use crate::factory::Factory;
use crate::model::{DockOperation, Layout, NodeId, NodeKind};
use crate::{Error, Result};

/// Drop zone within a target rect (edge bands + center).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropZone {
    Center,
    Left,
    Right,
    Top,
    Bottom,
}

impl DropZone {
    #[must_use]
    pub fn to_operation(self) -> DockOperation {
        match self {
            Self::Center => DockOperation::Fill,
            Self::Left => DockOperation::Left,
            Self::Right => DockOperation::Right,
            Self::Top => DockOperation::Top,
            Self::Bottom => DockOperation::Bottom,
        }
    }
}

/// Tab bar drop target registered during layout/update.
#[derive(Debug, Clone)]
pub struct TabBarTarget {
    pub pane: NodeId,
    pub bounds: Rectangle,
    /// Layout-space X for each insertion slot (same coordinates as tab [`Layout::bounds`]).
    pub insert_x: Vec<f32>,
    pub scroll_offset: f32,
}

/// Active tab drag session.
#[derive(Debug, Clone, Copy)]
pub struct DragSession {
    pub source_pane: NodeId,
    pub source_panel: NodeId,
    pub hover_target: Option<NodeId>,
    pub operation: Option<DockOperation>,
    pub tab_insert: Option<(NodeId, usize)>,
    /// Edge band size for content drop zones; matches [`crate::DropOverlayStyle::edge_fraction`].
    pub drop_edge_fraction: f32,
}

impl DragSession {
    #[must_use]
    pub fn new(source_pane: NodeId, source_panel: NodeId, drop_edge_fraction: f32) -> Self {
        Self {
            source_pane,
            source_panel,
            hover_target: None,
            operation: None,
            tab_insert: None,
            drop_edge_fraction: drop_edge_fraction.clamp(0.0, 0.5),
        }
    }
}

/// Validates and executes dock drops.
#[derive(Debug, Clone, Copy, Default)]
pub struct DockManager;

impl DockManager {
    fn pane_tab_count<K>(&self, layout: &Layout<K>, pane: NodeId) -> usize {
        match layout.kind(pane) {
            Some(NodeKind::Pane(p)) => p.tabs.len(),
            _ => 0,
        }
    }

    pub(crate) fn groups_compatible<K>(&self, layout: &Layout<K>, panel: NodeId, target_pane: NodeId) -> bool {
        let panel_group = match layout.kind(panel) {
            Some(NodeKind::Panel(p)) => p.group.as_deref(),
            _ => return false,
        };
        let pane_group = match layout.kind(target_pane) {
            Some(NodeKind::Pane(p)) => p.group.as_deref(),
            _ => return false,
        };
        match (panel_group, pane_group) {
            (Some(a), Some(b)) => a == b,
            _ => true,
        }
    }

    #[must_use]
    pub fn validate<K>(
        &self,
        layout: &Layout<K>,
        source_pane: NodeId,
        source_panel: NodeId,
        target: NodeId,
        op: DockOperation,
    ) -> bool {
        match op {
            DockOperation::Fill => {
                source_pane != target
                    && layout.is_leaf(source_panel)
                    && matches!(layout.kind(target), Some(NodeKind::Pane(_)))
                    && self.groups_compatible(layout, source_panel, target)
            }
            DockOperation::Left
            | DockOperation::Right
            | DockOperation::Top
            | DockOperation::Bottom => {
                self.is_target_visible(layout, target)
                    && matches!(layout.kind(target), Some(NodeKind::Pane(_)))
                    && (source_pane != target || self.pane_tab_count(layout, target) >= 1)
                    && (source_pane == target || self.is_target_visible(layout, source_pane))
            }
        }
    }

    #[must_use]
    pub fn is_target_visible<K>(&self, layout: &Layout<K>, target: NodeId) -> bool {
        matches!(
            layout.kind(target),
            Some(NodeKind::Panel(_) | NodeKind::Pane(_) | NodeKind::Proportional(_))
        )
    }

    pub fn execute<K>(&self, layout: &mut Layout<K>, session: DragSession) -> Result {
        let target = session.hover_target.ok_or(Error::MissingHoverTarget)?;
        let op = session.operation.ok_or(Error::MissingOperation)?;
        if !self.validate(
            layout,
            session.source_pane,
            session.source_panel,
            target,
            op,
        ) {
            return Err(Error::ValidationFailed);
        }
        let factory = Factory;
        if session.source_pane == target && op.is_edge() {
            factory.split_same_pane_edge(layout, target, session.source_panel, op)
        } else {
            match op {
                DockOperation::Fill => factory.dock_fill(layout, session.source_panel, target),
                _ => factory.split_cross_pane_edge(
                    layout,
                    session.source_pane,
                    session.source_panel,
                    target,
                    op,
                ),
            }
        }
    }

    /// Map pointer position inside `bounds` to a drop zone.
    ///
    /// `edge_fraction` is clamped to `0.0..=0.5` (same as [`crate::DropOverlayStyle::edge_fraction`]).
    #[must_use]
    pub fn hit_test_drop_zone(
        bounds: Rectangle,
        point: iced::Point,
        edge_fraction: f32,
    ) -> Option<DropZone> {
        if !bounds.contains(point) {
            return None;
        }
        let w = bounds.width;
        let h = bounds.height;
        if w <= 0.0 || h <= 0.0 {
            return None;
        }
        let edge = edge_fraction.clamp(0.0, 0.5);
        let rx = (point.x - bounds.x) / w;
        let ry = (point.y - bounds.y) / h;
        if rx < edge {
            Some(DropZone::Left)
        } else if rx > 1.0 - edge {
            Some(DropZone::Right)
        } else if ry < edge {
            Some(DropZone::Top)
        } else if ry > 1.0 - edge {
            Some(DropZone::Bottom)
        } else {
            Some(DropZone::Center)
        }
    }

    /// Find the pane under `point` and the drop zone within it.
    #[must_use]
    pub fn hit_test_pane(
        point: iced::Point,
        targets: &[(NodeId, Rectangle)],
        edge_fraction: f32,
    ) -> Option<(NodeId, DropZone)> {
        let mut best: Option<(NodeId, Rectangle, f32)> = None;
        for &(id, bounds) in targets {
            if bounds.contains(point) {
                let area = bounds.width * bounds.height;
                if best.is_none_or(|(_, _, a)| area < a) {
                    best = Some((id, bounds, area));
                }
            }
        }
        let (id, bounds, _) = best?;
        Self::hit_test_drop_zone(bounds, point, edge_fraction).map(|zone| (id, zone))
    }

    fn insertion_index_at(insert_x: &[f32], x: f32) -> usize {
        if insert_x.is_empty() {
            return 0;
        }
        if insert_x.len() == 1 {
            return 0;
        }
        for i in 0..insert_x.len() - 1 {
            let threshold = f32::midpoint(insert_x[i], insert_x[i + 1]);
            if x < threshold {
                return i;
            }
        }
        insert_x.len() - 1
    }

    /// Find the tab bar under `point` and the insertion index within it.
    #[must_use]
    pub fn hit_test_tab_insert(
        point: iced::Point,
        targets: &[TabBarTarget],
    ) -> Option<(NodeId, usize)> {
        let mut best: Option<(NodeId, f32, usize)> = None;
        for target in targets {
            if !target.bounds.contains(point) {
                continue;
            }
            let area = target.bounds.width * target.bounds.height;
            if best.is_none_or(|(_, a, _)| area < a) {
                let layout_x = point.x + target.scroll_offset;
                let index = Self::insertion_index_at(&target.insert_x, layout_x);
                best = Some((target.pane, area, index));
            }
        }
        best.map(|(pane, _, index)| (pane, index))
    }

    pub fn update_drag_hover(
        session: &mut DragSession,
        cursor: iced::Point,
        drop_targets: &[(NodeId, Rectangle)],
    ) {
        if let Some((target, zone)) =
            Self::hit_test_pane(cursor, drop_targets, session.drop_edge_fraction)
        {
            session.hover_target = Some(target);
            session.operation = Some(zone.to_operation());
        } else {
            session.hover_target = None;
            session.operation = None;
        }
    }

    /// Update hover state for an active drag; tab bar insertion takes priority over content zones.
    pub fn update_drag_hover_full(
        session: &mut DragSession,
        cursor: iced::Point,
        drop_targets: &[(NodeId, Rectangle)],
        tab_bar_targets: &[TabBarTarget],
    ) {
        if let Some((pane, index)) = Self::hit_test_tab_insert(cursor, tab_bar_targets) {
            session.tab_insert = Some((pane, index));
            session.hover_target = None;
            session.operation = None;
        } else {
            session.tab_insert = None;
            Self::update_drag_hover(session, cursor, drop_targets);
        }
    }

    pub fn execute_tab_insert<K>(
        &self,
        layout: &mut Layout<K>,
        session: DragSession,
        pane: NodeId,
        index: usize,
    ) -> Result {
        if !layout.is_leaf(session.source_panel) {
            return Err(Error::NotPanel {
                node: session.source_panel,
            });
        }
        if session.source_pane != pane && !self.groups_compatible(layout, session.source_panel, pane) {
            return Err(Error::ValidationFailed);
        }
        let factory = Factory;
        if session.source_pane == pane {
            factory.move_tab_in_pane(layout, pane, session.source_panel, index)
        } else {
            factory.move_panel_to_pane_at(layout, session.source_panel, pane, index)
        }
    }
}
