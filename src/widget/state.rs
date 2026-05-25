use std::cell::RefCell;
use std::rc::Rc;

use iced::Rectangle;

use crate::builder::compile::{build_tree, first_pane, BuiltLayout};
use crate::builder::DockIndex;
use crate::factory::Factory;
use crate::manager::{DockManager, DragSession, TabBarTarget};
use crate::model::{Layout as DockLayout, NodeId, NodeKind};
use crate::widget::action::{DockAction, TabAction};

/// Persistent docking state (stored in the widget [`Tree`](iced::advanced::widget::tree::Tree)).
#[derive(Debug, Clone)]
pub struct DockWidgetState<Theme = iced::Theme> {
    pub layout: DockLayout,
    pub index: DockIndex,
    pub drag: Option<DragSession>,
    pub drop_targets: Vec<(NodeId, Rectangle)>,
    pub tab_bar_targets: Vec<TabBarTarget>,
    /// Absolute bounds of each visible pane, collected each draw pass.
    pub pane_bounds: Vec<(NodeId, Rectangle)>,
    /// Pane that last received user focus (tab click or content click).
    pub focused_pane: Option<NodeId>,
    /// Set when focus changed without a layout rebuild; triggers a redraw.
    pub focus_dirty: bool,
    /// Set when the layout tree changes and the cached widget root must rebuild.
    pub layout_dirty: bool,
    /// Last iced theme from [`Widget::draw`]; survives per-frame dock widget rebuilds.
    pub resolved_theme: Rc<RefCell<Option<Theme>>>,
}

impl<Theme> DockWidgetState<Theme> {
    /// Rebuild string-id index from the current layout graph.
    pub fn sync_index(&mut self) {
        self.index = DockIndex::rebuild_from_layout(&self.layout);
    }

    pub(crate) fn commit_layout(&mut self) {
        if self.layout_dirty {
            self.sync_index();
            self.layout_dirty = false;
        }
    }

    /// Build widget state from a declarative [`LayoutTree`](crate::LayoutTree).
    pub fn from_tree(tree: crate::LayoutTree) -> crate::Result<Self> {
        let built = build_tree(&tree)?;
        let focused_pane = first_pane(&built.layout);
        Ok(Self::from_built(built, focused_pane))
    }

    /// Build widget state from a compiled layout.
    pub fn from_built(
        built: BuiltLayout,
        focused_pane: Option<NodeId>,
    ) -> Self {
        Self {
            layout: built.layout,
            index: built.index,
            drag: None,
            drop_targets: Vec::new(),
            tab_bar_targets: Vec::new(),
            pane_bounds: Vec::new(),
            focused_pane,
            focus_dirty: false,
            layout_dirty: true,
            resolved_theme: Rc::new(RefCell::new(None)),
        }
    }
}

impl<Theme> Default for DockWidgetState<Theme> {
    fn default() -> Self {
        let layout = DockLayout::new();
        let index = DockIndex::rebuild_from_layout(&layout);
        Self {
            layout,
            index,
            drag: None,
            drop_targets: Vec::new(),
            tab_bar_targets: Vec::new(),
            pane_bounds: Vec::new(),
            focused_pane: None,
            focus_dirty: false,
            layout_dirty: false,
            resolved_theme: Rc::new(RefCell::new(None)),
        }
    }
}

/// End an active drag at `cursor`, applying a drop when valid.
pub fn finish_drag<Theme>(state: &mut DockWidgetState<Theme>, cursor: Option<iced::Point>) -> bool {
    let Some(cursor) = cursor else {
        let had_drag = state.drag.is_some();
        state.drag = None;
        return had_drag;
    };

    let drop_targets = state.drop_targets.clone();
    let tab_bar_targets = state.tab_bar_targets.clone();
    let Some(session) = state.drag.take() else {
        return false;
    };

    let mut session = session;
    DockManager::update_drag_hover_full(&mut session, cursor, &drop_targets, &tab_bar_targets);
    let mut changed = false;
    if let Some((pane, index)) = session.tab_insert {
        if DockManager
            .execute_tab_insert(&mut state.layout, session, pane, index)
            .is_ok()
        {
            state.layout_dirty = true;
            changed = true;
        }
    } else if DockManager.execute(&mut state.layout, session).is_ok() {
        state.layout_dirty = true;
        changed = true;
    }
    if changed {
        state.sync_index();
    }
    changed
}

/// Apply a [`DockAction`] to dock state (programmatic / session API).
///
/// Does not emit [`DockEvent`] values. After a successful structural change, call
/// [`DockWidgetState::sync_index`] or rely on the widget's next layout pass.
pub fn dispatch_action<Theme>(state: &mut DockWidgetState<Theme>, action: DockAction) -> bool {
    let factory = Factory;
    let mut changed = false;

    match action {
        DockAction::Tab(tab_msg) => match tab_msg {
            TabAction::Select { pane, panel } => {
                factory.set_active_panel(&mut state.layout, pane, panel);
                if state.focused_pane != Some(pane) {
                    state.focused_pane = Some(pane);
                    state.focus_dirty = true;
                }
                state.layout_dirty = true;
                changed = true;
            }
            TabAction::Close { panel } => {
                if factory.close(&mut state.layout, panel).is_ok() {
                    if let Some(id) = state
                        .index
                        .panels
                        .iter()
                        .find_map(|(s, &n)| (n == panel).then(|| s.clone()))
                    {
                        state.index.panels.remove(&id);
                    }
                    state.layout_dirty = true;
                    changed = true;
                }
            }
            TabAction::DragStarted {
                source_pane,
                source_panel,
                drop_edge_fraction,
            } => {
                state.drag = Some(DragSession::new(
                    source_pane,
                    source_panel,
                    drop_edge_fraction,
                ));
                state.layout_dirty = true;
                changed = true;
            }
            TabAction::DragEnded { cursor } => {
                if finish_drag(state, Some(cursor)) {
                    changed = true;
                }
            }
            TabAction::DragMoved { cursor } => {
                let drop_targets = state.drop_targets.clone();
                let tab_bar_targets = state.tab_bar_targets.clone();
                if let Some(ref mut session) = state.drag {
                    DockManager::update_drag_hover_full(
                        session,
                        cursor,
                        &drop_targets,
                        &tab_bar_targets,
                    );
                }
            }
            TabAction::DragCancelled => {
                state.drag = None;
                state.layout_dirty = true;
                changed = true;
            }
        },
        DockAction::PaneFocused { pane, panel } => {
            if let Some(panel_node) = panel {
                let tab_changed = matches!(
                    state.layout.kind(pane),
                    Some(NodeKind::Pane(p)) if p.active != Some(panel_node)
                );
                if tab_changed {
                    factory.set_active_panel(&mut state.layout, pane, panel_node);
                    state.layout_dirty = true;
                    changed = true;
                }
            }
            if state.focused_pane != Some(pane) {
                state.focused_pane = Some(pane);
                state.focus_dirty = true;
                changed = true;
            }
        }
        DockAction::SplitDrag {
            group,
            splitter_index,
            pair_ratio,
        } => {
            if factory
                .adjust_splitter(&mut state.layout, group, splitter_index, pair_ratio)
                .is_ok()
            {
                state.layout_dirty = true;
                changed = true;
            }
        }
    }
    if changed && state.layout_dirty {
        state.sync_index();
    }
    changed
}
