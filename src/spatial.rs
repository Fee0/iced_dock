//! Spatial helpers for pane navigation (keyboard focus movement).

use std::collections::HashMap;

use crate::model::NodeId;

/// Cardinal direction for adjacent-pane lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

fn overlaps_perpendicular(a: iced::Rectangle, b: iced::Rectangle, horizontal: bool) -> bool {
    if horizontal {
        a.y < b.y + b.height && b.y < a.y + a.height
    } else {
        a.x < b.x + b.width && b.x < a.x + a.width
    }
}

/// Find the nearest pane in `direction` from `pane`.
///
/// `pane_bounds` maps each visible pane id to its absolute screen bounds (collected each
/// draw pass). Unlike a single-point probe, this tolerates splitter gaps between panes.
#[must_use]
pub fn adjacent_pane(
    pane: NodeId,
    direction: Direction,
    pane_bounds: &HashMap<NodeId, iced::Rectangle>,
) -> Option<NodeId> {
    let current = pane_bounds.get(&pane)?;
    let horizontal = matches!(direction, Direction::Left | Direction::Right);

    let mut best: Option<(NodeId, f32)> = None;

    for (&id, region) in pane_bounds {
        if id == pane || !overlaps_perpendicular(*current, *region, horizontal) {
            continue;
        }

        let distance = match direction {
            Direction::Left => current.x - (region.x + region.width),
            Direction::Right => region.x - (current.x + current.width),
            Direction::Up => current.y - (region.y + region.height),
            Direction::Down => region.y - (current.y + current.height),
        };

        if distance < 0.0 {
            continue;
        }

        if best.is_none_or(|(_, best_dist)| distance < best_dist) {
            best = Some((id, distance));
        }
    }

    best.map(|(id, _)| id)
}

/// Build a map from the transient `(NodeId, Rectangle)` list collected during draw.
#[must_use]
pub fn pane_bounds_map(
    pane_bounds: &[(NodeId, iced::Rectangle)],
) -> HashMap<NodeId, iced::Rectangle> {
    pane_bounds.iter().copied().collect()
}
