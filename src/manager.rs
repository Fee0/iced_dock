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
    pub source_group: NodeId,
    pub source_tab: NodeId,
    pub hover_target: Option<NodeId>,
    pub operation: Option<DockOperation>,
}

impl DragSession {
    pub fn new(source_group: NodeId, source_tab: NodeId) -> Self {
        Self {
            source_group,
            source_tab,
            hover_target: None,
            operation: None,
        }
    }
}

/// Validates and executes dock drops.
#[derive(Debug, Clone, Copy, Default)]
pub struct DockManager;

impl DockManager {
    pub fn validate(
        &self,
        layout: &Layout,
        source_group: NodeId,
        source_tab: NodeId,
        target: NodeId,
        op: DockOperation,
    ) -> bool {
        match op {
            DockOperation::Fill => {
                source_group != target
                    && layout.is_leaf(source_tab)
                    && matches!(layout.kind(target), Some(NodeKind::TabGroup(_)))
            }
            DockOperation::Left
            | DockOperation::Right
            | DockOperation::Top
            | DockOperation::Bottom => {
                source_group != target
                    && self.is_target_visible(layout, source_group)
                    && self.is_target_visible(layout, target)
            }
        }
    }

    pub fn is_target_visible(&self, layout: &Layout, target: NodeId) -> bool {
        matches!(
            layout.kind(target),
            Some(
                NodeKind::Document(_)
                    | NodeKind::Tool(_)
                    | NodeKind::TabGroup(_)
                    | NodeKind::Proportional(_)
            )
        )
    }

    pub fn execute(
        &self,
        layout: &mut Layout,
        session: DragSession,
    ) -> Result<(), ()> {
        let target = session.hover_target.ok_or(())?;
        let op = session.operation.ok_or(())?;
        if !self.validate(
            layout,
            session.source_group,
            session.source_tab,
            target,
            op,
        ) {
            return Err(());
        }
        let factory = Factory;
        match op {
            DockOperation::Fill => factory.dock_fill(layout, session.source_tab, target),
            // Edge drops move the entire source tab group (pane), not only the active tab.
            _ => factory.split(layout, session.source_group, target, op),
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
}
