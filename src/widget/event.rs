//! Observation events emitted to the application (string panel / pane ids).

use crate::builder::DockIndex;
use crate::model::{Layout, NodeId, NodeKind};
use crate::widget::action::{DockAction, TabAction};

/// User-visible dock notification. The widget applies layout changes before this is delivered;
/// do not call [`DockSession::dispatch`](crate::DockSession::dispatch) for widget-originated input.
#[derive(Debug, Clone, PartialEq)]
pub enum DockEvent {
    TabSelected {
        pane: Option<String>,
        panel: String,
    },
    TabClosed {
        panel: String,
    },
    PaneFocused {
        pane: Option<String>,
        panel: Option<String>,
    },
    SplitResized {
        splitter_index: usize,
        pair_ratio: f32,
    },
    DragStarted {
        panel: String,
    },
    DragMoved {
        cursor: iced::Point,
    },
    DragEnded {
        cursor: iced::Point,
    },
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
