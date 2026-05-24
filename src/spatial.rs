//! Spatial helpers for pane navigation (keyboard focus movement).

use std::collections::HashMap;

use iced::Point;

use crate::model::NodeId;

/// Cardinal direction for adjacent-pane lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

/// Find the pane whose bounds contain a probe point just outside `pane` in `direction`.
///
/// `pane_bounds` maps each visible pane id to its absolute screen bounds (collected each
/// layout pass). Same geometric approach as iced's `pane_grid::State::adjacent`.
pub fn adjacent_pane(
    pane: NodeId,
    direction: Direction,
    pane_bounds: &HashMap<NodeId, iced::Rectangle>,
) -> Option<NodeId> {
    let current = pane_bounds.get(&pane)?;

    let target = match direction {
        Direction::Left => Point::new(current.x - 1.0, current.y + 1.0),
        Direction::Right => Point::new(current.x + current.width + 1.0, current.y + 1.0),
        Direction::Up => Point::new(current.x + 1.0, current.y - 1.0),
        Direction::Down => Point::new(current.x + 1.0, current.y + current.height + 1.0),
    };

    pane_bounds
        .iter()
        .find(|(id, region)| **id != pane && region.contains(target))
        .map(|(id, _)| *id)
}

/// Build a map from the transient `(NodeId, Rectangle)` list collected during layout.
pub fn pane_bounds_map(pane_bounds: &[(NodeId, iced::Rectangle)]) -> HashMap<NodeId, iced::Rectangle> {
    pane_bounds.iter().copied().collect()
}
