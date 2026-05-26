//! Internal dock commands (stable [`NodeId`] handles). Applied by the widget or [`DockSession::dispatch`].

use crate::model::NodeId;

/// A tab-level action within a dock pane.
///
/// These are produced by the dock widget in response to user interaction
/// (clicking tabs, closing tabs, dragging tabs between panes).
#[derive(Debug, Clone)]
pub enum TabAction {
    /// Activate a tab within its pane.
    Select {
        /// The pane containing the tab.
        pane: NodeId,
        /// The panel (tab) to activate.
        panel: NodeId,
    },
    /// Close a tab, removing it from its pane.
    Close {
        /// The panel (tab) to close.
        panel: NodeId,
    },
    /// A tab label drag has started (pointer moved past the drag threshold).
    DragStarted {
        /// The pane the dragged tab belongs to.
        source_pane: NodeId,
        /// The panel being dragged.
        source_panel: NodeId,
        /// Fraction of pane edge used for directional drop zones.
        drop_edge_fraction: f32,
    },
    /// The pointer moved while a tab drag is active.
    DragMoved {
        /// Current pointer position.
        cursor: iced::Point,
    },
    /// The pointer was released, ending a tab drag.
    DragEnded {
        /// Pointer position at the moment of release.
        cursor: iced::Point,
    },
    /// The tab drag was cancelled (e.g. pointer left the window).
    DragCancelled,
}

/// A dock-level action dispatched by the widget or programmatically via
/// [`DockSession::dispatch`](crate::DockSession::dispatch).
#[derive(Debug, Clone)]
pub enum DockAction {
    /// A tab-level sub-action (select, close, drag).
    Tab(TabAction),
    /// User clicked into a pane's content area (or app requested pane focus).
    PaneFocused {
        /// The pane that received focus.
        pane: NodeId,
        /// When `Some`, activates that tab before updating pane focus.
        panel: Option<NodeId>,
    },
    /// A splitter handle was dragged, resizing two adjacent panes.
    SplitDrag {
        /// The proportional split group containing the splitter.
        group: NodeId,
        /// Zero-based index of the splitter within the group.
        splitter_index: usize,
        /// Fraction of the adjacent pair's space allocated to the left/top pane.
        pair_ratio: f32,
    },
}
