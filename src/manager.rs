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
    pub source: NodeId,
    pub hover_target: Option<NodeId>,
    pub operation: Option<DockOperation>,
}

impl DragSession {
    pub fn new(source: NodeId) -> Self {
        Self {
            source,
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
        source: NodeId,
        target: NodeId,
        op: DockOperation,
    ) -> bool {
        if source == target {
            return false;
        }

        match op {
            DockOperation::Fill => {
                let source_kind = layout.leaf_kind(source);
                let target_kind = layout.tab_group_kind(target);
                source_kind.is_some() && target_kind == source_kind
            }
            DockOperation::Left
            | DockOperation::Right
            | DockOperation::Top
            | DockOperation::Bottom => {
                self.is_target_visible(layout, source)
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
        if !self.validate(layout, session.source, target, op) {
            return Err(());
        }
        let factory = Factory;
        match op {
            DockOperation::Fill => factory.dock_fill(layout, session.source, target),
            _ => factory.split(layout, session.source, target, op),
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
