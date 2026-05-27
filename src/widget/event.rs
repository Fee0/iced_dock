//! Observation events emitted to the application.

use crate::model::{Layout, NodeId, NodeKind};
use crate::widget::action::{DockAction, TabAction};

/// User-visible dock notification. The widget applies layout changes before this is delivered;
/// do not call [`DockSession::dispatch`](crate::DockSession::dispatch) for widget-originated input.
///
/// Map these to your application message type via
/// [`DockBuilder::on_event`](crate::widget::DockBuilder::on_event).
#[derive(Debug, Clone, PartialEq)]
pub enum DockEvent<K> {
    /// A tab was activated within a pane.
    TabSelected {
        /// String name of the containing pane, if one was set.
        pane: Option<String>,
        /// Content key of the selected panel.
        panel: K,
    },
    /// A tab was closed.
    TabClosed {
        /// Content key of the closed panel.
        panel: K,
    },
    /// A pane received user focus (content click or tab select).
    PaneFocused {
        /// String name of the focused pane, if one was set.
        pane: Option<String>,
        /// Content key of the active panel in the focused pane, if any.
        panel: Option<K>,
    },
    /// A splitter between two panes was dragged.
    SplitResized {
        /// Zero-based index of the splitter within its split group.
        splitter_index: usize,
        /// New fraction of the adjacent pair's space allocated to the left/top pane.
        pair_ratio: f32,
    },
    /// A tab drag gesture started.
    DragStarted {
        /// Content key of the panel being dragged.
        panel: K,
    },
    /// The pointer moved during a tab drag.
    DragMoved {
        /// Current pointer position.
        cursor: iced::Point,
    },
    /// The pointer was released, ending a tab drag.
    DragEnded {
        /// Pointer position at the moment of release.
        cursor: iced::Point,
    },
    /// The tab drag was cancelled.
    DragCancelled,
    /// Structural layout change (tab close, dock drop, split resize, etc.).
    LayoutChanged,
}

/// Map an applied [`DockAction`] to a public [`DockEvent`], if any.
pub fn action_to_event<K: Clone>(layout: &Layout<K>, action: &DockAction) -> Option<DockEvent<K>> {
    match action {
        DockAction::Tab(tab) => tab_action_to_event(layout, tab),
        DockAction::PaneFocused { pane, panel } => Some(DockEvent::PaneFocused {
            pane: pane_name(layout, *pane),
            panel: panel.and_then(|p| panel_key(layout, p)),
        }),
        DockAction::SplitDrag {
            splitter_index,
            pair_ratio,
            ..
        } => Some(DockEvent::SplitResized {
            splitter_index: *splitter_index,
            pair_ratio: *pair_ratio,
        }),
    }
}

fn tab_action_to_event<K: Clone>(layout: &Layout<K>, action: &TabAction) -> Option<DockEvent<K>> {
    match action {
        TabAction::Select { pane, panel } => Some(DockEvent::TabSelected {
            pane: pane_name(layout, *pane),
            panel: panel_key(layout, *panel)?,
        }),
        TabAction::Close { panel } => Some(DockEvent::TabClosed {
            panel: panel_key(layout, *panel)?,
        }),
        TabAction::DragStarted { source_panel, .. } => Some(DockEvent::DragStarted {
            panel: panel_key(layout, *source_panel)?,
        }),
        TabAction::DragMoved { cursor } => Some(DockEvent::DragMoved { cursor: *cursor }),
        TabAction::DragEnded { cursor } => Some(DockEvent::DragEnded { cursor: *cursor }),
        TabAction::DragCancelled => Some(DockEvent::DragCancelled),
    }
}

fn panel_key<K: Clone>(layout: &Layout<K>, panel: NodeId) -> Option<K> {
    match layout.kind(panel)? {
        NodeKind::Panel(p) => Some(p.content.clone()),
        _ => None,
    }
}

fn pane_name<K>(layout: &Layout<K>, pane: NodeId) -> Option<String> {
    match layout.kind(pane)? {
        NodeKind::Pane(p) => p.name.clone(),
        _ => None,
    }
}
