use super::{Axis, NodeId};

/// Ordered split container (rendered as nested `iced_split` widgets).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ProportionalGroup {
    pub axis: Axis,
    pub children: Vec<NodeId>,
    /// Relative weights, same length as `children`.
    pub proportions: Vec<f32>,
}

impl ProportionalGroup {
    #[must_use]
    pub fn new(axis: Axis, children: Vec<NodeId>) -> Self {
        let n = children.len();
        let proportions = if n == 0 { Vec::new() } else { vec![1.0; n] };
        Self {
            axis,
            children,
            proportions,
        }
    }

    pub fn normalize_proportions(&mut self) {
        let sum: f32 = self.proportions.iter().sum();
        if sum > 0.0 {
            for p in &mut self.proportions {
                *p /= sum;
            }
        }
    }
}
