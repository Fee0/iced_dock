//! Drag validation and drop execution.

use iced::Rectangle;

use crate::factory::Factory;
use crate::model::{DockOperation, Layout, NodeId, NodeKind};

/// Drop zone within a target rect (20% edge bands + center).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropZone {
    Center,
    Left,
    Right,
    Top,
    Bottom,
}

impl DropZone {
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

/// Active tab drag session.
#[derive(Debug, Clone, Copy)]
pub struct DragSession {
    pub source_pane: NodeId,
    pub source_panel: NodeId,
    pub hover_target: Option<NodeId>,
    pub operation: Option<DockOperation>,
}

impl DragSession {
    pub fn new(source_pane: NodeId, source_panel: NodeId) -> Self {
        Self {
            source_pane,
            source_panel,
            hover_target: None,
            operation: None,
        }
    }
}

/// Validates and executes dock drops.
#[derive(Debug, Clone, Copy, Default)]
pub struct DockManager;

impl DockManager {
    fn pane_tab_count(&self, layout: &Layout, pane: NodeId) -> usize {
        match layout.kind(pane) {
            Some(NodeKind::Pane(p)) => p.tabs.len(),
            _ => 0,
        }
    }

    pub fn validate(
        &self,
        layout: &Layout,
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

    pub fn is_target_visible(&self, layout: &Layout, target: NodeId) -> bool {
        matches!(
            layout.kind(target),
            Some(NodeKind::Panel(_) | NodeKind::Pane(_) | NodeKind::Proportional(_))
        )
    }

    pub fn execute(&self, layout: &mut Layout, session: DragSession) -> Result<(), ()> {
        let target = session.hover_target.ok_or(())?;
        let op = session.operation.ok_or(())?;
        if !self.validate(
            layout,
            session.source_pane,
            session.source_panel,
            target,
            op,
        ) {
            return Err(());
        }
        let factory = Factory;
        if session.source_pane == target && op.is_edge() {
            factory.split_same_pane_edge(layout, target, session.source_panel, op)
        } else {
            match op {
                DockOperation::Fill => {
                    factory.dock_fill(layout, session.source_panel, target)
                }
                _ => factory.split(layout, session.source_pane, target, op),
            }
        }
    }

    /// Map pointer position inside `bounds` to a drop zone.
    pub fn hit_test_drop_zone(bounds: Rectangle, point: iced::Point) -> Option<DropZone> {
        if !bounds.contains(point) {
            return None;
        }
        let w = bounds.width;
        let h = bounds.height;
        if w <= 0.0 || h <= 0.0 {
            return None;
        }
        let rx = (point.x - bounds.x) / w;
        let ry = (point.y - bounds.y) / h;
        const EDGE: f32 = 0.2;
        if rx < EDGE {
            Some(DropZone::Left)
        } else if rx > 1.0 - EDGE {
            Some(DropZone::Right)
        } else if ry < EDGE {
            Some(DropZone::Top)
        } else if ry > 1.0 - EDGE {
            Some(DropZone::Bottom)
        } else {
            Some(DropZone::Center)
        }
    }

    /// Find the pane under `point` and the drop zone within it.
    pub fn hit_test_pane(
        point: iced::Point,
        targets: &[(NodeId, Rectangle)],
    ) -> Option<(NodeId, DropZone)> {
        let mut best: Option<(NodeId, Rectangle, f32)> = None;
        for &(id, bounds) in targets {
            if bounds.contains(point) {
                let area = bounds.width * bounds.height;
                if best.map(|(_, _, a)| area < a).unwrap_or(true) {
                    best = Some((id, bounds, area));
                }
            }
        }
        best.and_then(|(id, bounds, _)| {
            Self::hit_test_drop_zone(bounds, point).map(|zone| (id, zone))
        })
    }

    pub fn update_drag_hover(
        session: &mut DragSession,
        cursor: iced::Point,
        drop_targets: &[(NodeId, Rectangle)],
    ) {
        if let Some((target, zone)) = Self::hit_test_pane(cursor, drop_targets) {
            session.hover_target = Some(target);
            session.operation = Some(zone.to_operation());
        } else {
            session.hover_target = None;
            session.operation = None;
        }
    }
}
