//! Observation events emitted to the application (string panel / pane ids).

use crate::builder::DockIndex;
use crate::model::{Layout, NodeId, NodeKind};
use crate::widget::action::{DockAction, TabAction};

/// User-visible dock notification. The widget applies layout changes before this is delivered;
/// do not call [`DockSession::dispatch`](crate::DockSession::dispatch) for widget-originated input.
///
/// Map these to your application message type via
/// [`DockBuilder::on_event`](crate::widget::DockBuilder::on_event).
#[derive(Debug, Clone, PartialEq)]
pub enum DockEvent {
    /// A tab was activated within a pane.
    TabSelected {
        /// String name of the containing pane, if one was set.
        pane: Option<String>,
        /// String id of the selected panel.
        panel: String,
    },
    /// A tab was closed.
    TabClosed {
        /// String id of the closed panel.
        panel: String,
    },
    /// A pane received user focus (content click or tab select).
    PaneFocused {
        /// String name of the focused pane, if one was set.
        pane: Option<String>,
        /// String id of the active panel in the focused pane, if any.
        panel: Option<String>,
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
        /// String id of the panel being dragged.
        panel: String,
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
pub fn action_to_event<K>(
    layout: &Layout<K>,
    index: &DockIndex,
    action: &DockAction,
) -> Option<DockEvent> {
    match action {
        DockAction::Tab(tab) => tab_action_to_event(layout, index, tab),
        DockAction::PaneFocused { pane, panel } => Some(DockEvent::PaneFocused {
            pane: pane_name(layout, *pane),
            panel: panel.and_then(|p| panel_id(layout, index, p)),
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

fn tab_action_to_event<K>(
    layout: &Layout<K>,
    index: &DockIndex,
    action: &TabAction,
) -> Option<DockEvent> {
    match action {
        TabAction::Select { pane, panel } => Some(DockEvent::TabSelected {
            pane: pane_name(layout, *pane),
            panel: panel_id(layout, index, *panel)?,
        }),
        TabAction::Close { panel } => Some(DockEvent::TabClosed {
            panel: panel_id(layout, index, *panel)?,
        }),
        TabAction::DragStarted { source_panel, .. } => Some(DockEvent::DragStarted {
            panel: panel_id(layout, index, *source_panel)?,
        }),
        TabAction::DragMoved { cursor } => Some(DockEvent::DragMoved { cursor: *cursor }),
        TabAction::DragEnded { cursor } => Some(DockEvent::DragEnded { cursor: *cursor }),
        TabAction::DragCancelled => Some(DockEvent::DragCancelled),
    }
}

pub(crate) fn panel_id<K>(layout: &Layout<K>, index: &DockIndex, panel: NodeId) -> Option<String> {
    index
        .panels
        .iter()
        .find_map(|(id, node)| (*node == panel).then(|| id.clone()))
        .or_else(|| {
            let e = layout.get(panel)?;
            match &e.kind {
                NodeKind::Panel(p) => Some(p.id.clone()),
                _ => None,
            }
        })
}

fn pane_name<K>(layout: &Layout<K>, pane: NodeId) -> Option<String> {
    match layout.kind(pane)? {
        NodeKind::Pane(p) => p.name.clone(),
        _ => None,
    }
}
