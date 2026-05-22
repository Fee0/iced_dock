//! Nested `iced_split` folding for proportional groups.

use iced::Element;
use iced_split::{Direction, Split, Strategy};

use crate::model::{Axis, NodeId, ProportionalGroup};

/// Fold N children into nested binary splits.
pub fn fold_proportional<'a, Message: Clone + 'static>(
    group: &ProportionalGroup,
    mut child_builder: impl FnMut(NodeId) -> Element<'a, Message>,
    on_split_drag: impl Fn(f32) -> Message + Clone + 'static,
) -> Element<'a, Message> {
    let n = group.children.len();
    if n == 0 {
        return iced::widget::text("").into();
    }
    if n == 1 {
        return child_builder(group.children[0]);
    }

    let direction = match group.axis {
        Axis::Horizontal => Direction::Vertical,
        Axis::Vertical => Direction::Horizontal,
    };

    let mut elements: Vec<Element<Message>> = group
        .children
        .iter()
        .map(|&id| child_builder(id))
        .collect();

    let proportions = &group.proportions;

    while elements.len() > 1 {
        let right = elements.pop().unwrap();
        let left = elements.pop().unwrap();
        let idx = elements.len();
        let left_weight = proportions.get(idx).copied().unwrap_or(1.0);
        let right_weight = proportions.get(idx + 1).copied().unwrap_or(1.0);
        let sum = left_weight + right_weight;
        let split_at = if sum > 0.0 {
            left_weight / sum
        } else {
            0.5
        };

        let on_drag = on_split_drag.clone();
        let merged = Split::new(left, right, split_at)
            .direction(direction)
            .strategy(Strategy::Relative)
            .on_drag(move |ratio: f32| on_drag(ratio));
        elements.push(merged.into());
    }

    elements.pop().unwrap()
}
