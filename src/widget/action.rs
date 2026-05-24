//! Internal dock commands (stable [`NodeId`] handles). Applied by the widget or [`DockSession::dispatch`].

use crate::model::NodeId;

#[derive(Debug, Clone)]
pub enum TabAction {
    Select {
        pane: NodeId,
        panel: NodeId,
    },
    Close {
        panel: NodeId,
    },
    DragStarted {
        source_pane: NodeId,
        source_panel: NodeId,
    },
    DragMoved {
        cursor: iced::Point,
    },
    DragEnded {
        cursor: iced::Point,
    },
    DragCancelled,
}

#[derive(Debug, Clone)]
pub enum DockAction {
    Tab(TabAction),
    /// User clicked into a pane's content area (or app requested pane focus).
    PaneFocused {
        pane: NodeId,
        /// When `Some`, activates that tab before updating pane focus.
        panel: Option<NodeId>,
    },
    SplitDrag {
        group: NodeId,
        splitter_index: usize,
        /// Fraction of the adjacent pair's space allocated to the left/top pane.
        pair_ratio: f32,
    },
}
