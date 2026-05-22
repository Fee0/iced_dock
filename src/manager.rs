//! Drag validation and docking execution (Dock `DockManager` port).

use crate::factory::{Factory, FactoryError};
use crate::model::{DockOperation, Layout, NodeId, NodeKind};

/// Active drag state.
#[derive(Debug, Clone)]
pub struct DragSession {
    pub source: NodeId,
    pub hover_target: Option<NodeId>,
    pub operation: DockOperation,
}

/// Docking manager options.
#[derive(Debug, Clone)]
pub struct DockManager {
    pub docking_enabled: bool,
}

impl Default for DockManager {
    fn default() -> Self {
        Self {
            docking_enabled: true,
        }
    }
}

impl DockManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_target_visible(
        &self,
        layout: &Layout,
        source: NodeId,
        target: NodeId,
        operation: DockOperation,
    ) -> bool {
        if !self.docking_enabled {
            return false;
        }
        self.validate(layout, source, target, operation, false)
            .is_ok()
    }

    pub fn validate(
        &self,
        layout: &Layout,
        source: NodeId,
        target: NodeId,
        operation: DockOperation,
        _execute: bool,
    ) -> Result<(), DockError> {
        if !self.docking_enabled {
            return Err(DockError::Disabled);
        }

        let source_kind = layout.leaf_kind(source).ok_or(DockError::InvalidSource)?;

        let can_drag = match layout.kind(source) {
            Some(NodeKind::Document(m) | NodeKind::Tool(m)) => m.can_drag,
            _ => return Err(DockError::InvalidSource),
        };
        if !can_drag {
            return Err(DockError::NotAllowed);
        }

        match operation {
            DockOperation::Fill => {
                let target_kind = layout.tab_group_kind(target).ok_or(DockError::InvalidTarget)?;
                if target_kind != source_kind {
                    return Err(DockError::KindMismatch);
                }
                let can_drop = self.target_accepts_drop(layout, target);
                if !can_drop {
                    return Err(DockError::NotAllowed);
                }
            }
            DockOperation::Left
            | DockOperation::Right
            | DockOperation::Top
            | DockOperation::Bottom => {
                if !self.can_split_target(layout, target) {
                    return Err(DockError::InvalidTarget);
                }
            }
        }

        Ok(())
    }

    pub fn execute(
        &self,
        factory: &Factory,
        layout: &mut Layout,
        session: &DragSession,
    ) -> Result<(), DockError> {
        self.validate(
            layout,
            session.source,
            session.hover_target.ok_or(DockError::InvalidTarget)?,
            session.operation,
            true,
        )?;

        let target = session.hover_target.unwrap();
        let source = session.source;

        match session.operation {
            DockOperation::Fill => {
                factory
                    .dock_fill(layout, source, target)
                    .map_err(DockError::Factory)?;
            }
            op @ (DockOperation::Left
            | DockOperation::Right
            | DockOperation::Top
            | DockOperation::Bottom) => {
                let split_target = self.resolve_split_target(layout, target, op);
                factory
                    .split(layout, split_target, source, op)
                    .map_err(DockError::Factory)?;
            }
        }

        Ok(())
    }

    fn target_accepts_drop(&self, layout: &Layout, target: NodeId) -> bool {
        match layout.kind(target) {
            Some(NodeKind::TabGroup(_)) => true,
            _ => false,
        }
    }

    fn can_split_target(&self, layout: &Layout, target: NodeId) -> bool {
        matches!(
            layout.kind(target),
            Some(
                NodeKind::TabGroup(_)
                    | NodeKind::Proportional(_)
                    | NodeKind::Document(_)
                    | NodeKind::Tool(_)
            )
        )
    }

    /// Split relative to tab group or leaf; proportional groups split the group itself.
    fn resolve_split_target(&self, layout: &Layout, target: NodeId, _op: DockOperation) -> NodeId {
        if let Some(NodeKind::TabGroup(g)) = layout.kind(target) {
            return g.active.unwrap_or(target);
        }
        target
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DockError {
    Disabled,
    InvalidSource,
    InvalidTarget,
    KindMismatch,
    NotAllowed,
    Factory(FactoryError),
}

impl From<FactoryError> for DockError {
    fn from(e: FactoryError) -> Self {
        DockError::Factory(e)
    }
}

/// Hit zone within a rectangle for drop targeting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropZone {
    Left,
    Right,
    Top,
    Bottom,
    Center,
}

impl DropZone {
    pub fn to_operation(self) -> DockOperation {
        match self {
            Self::Left => DockOperation::Left,
            Self::Right => DockOperation::Right,
            Self::Top => DockOperation::Top,
            Self::Bottom => DockOperation::Bottom,
            Self::Center => DockOperation::Fill,
        }
    }
}

/// Pick drop zone from pointer position inside bounds (20% edge bands).
pub fn hit_test_drop_zone(
    bounds: (f32, f32, f32, f32),
    x: f32,
    y: f32,
) -> DropZone {
    let (bx, by, bw, bh) = bounds;
    let lx = x - bx;
    let ly = y - by;
    let edge_x = bw * 0.2;
    let edge_y = bh * 0.2;

    if lx < edge_x {
        DropZone::Left
    } else if lx > bw - edge_x {
        DropZone::Right
    } else if ly < edge_y {
        DropZone::Top
    } else if ly > bh - edge_y {
        DropZone::Bottom
    } else {
        DropZone::Center
    }
}
