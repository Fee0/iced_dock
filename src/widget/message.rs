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
    SplitDrag {
        group: NodeId,
        splitter_index: usize,
        ratio: f32,
    },
    LayoutChanged,
}
