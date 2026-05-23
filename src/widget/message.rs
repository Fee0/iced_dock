use crate::manager::DropZone;
use crate::model::NodeId;

#[derive(Debug, Clone)]
pub enum TabMessage {
    Select { group: NodeId, tab: NodeId },
    Close { tab: NodeId },
    DragStarted {
        source_group: NodeId,
        source_tab: NodeId,
    },
    DragMoved {
        target: NodeId,
        zone: DropZone,
    },
    DragEnded {
        target: NodeId,
        zone: DropZone,
    },
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
