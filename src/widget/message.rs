use crate::model::NodeId;

#[derive(Debug, Clone)]
pub enum TabMessage {
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
pub enum DockMessage {
    Tab(TabMessage),
    /// User clicked into a pane's content area (or app requested pane focus).
    PaneFocused {
        pane: NodeId,
        /// The pane's currently active tab, if any.
        panel: Option<NodeId>,
    },
    SplitDrag {
        group: NodeId,
        splitter_index: usize,
        /// Fraction of the adjacent pair's space allocated to the left/top pane.
        pair_ratio: f32,
    },
    LayoutChanged,
}
