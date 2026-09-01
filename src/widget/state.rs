use std::collections::HashSet;

use iced::Rectangle;

use crate::builder::compile::{build_tree, first_pane, first_pane_where, BuiltLayout};
use crate::builder::DockIndex;
use crate::factory::Factory;
use crate::manager::{DockManager, DragSession, TabBarTarget};
use crate::model::{Layout as DockLayout, NodeId, NodeKind, Pane};
use crate::widget::action::{DockAction, TabAction};

/// Persistent docking state shared between the [`Dock`](crate::Dock) widget
/// and the application via `Rc<RefCell<DockWidgetState<K>>>`.
///
/// Obtain one from [`DockSession::state`](crate::DockSession::state) or
/// [`DockWidgetState::from_tree`].
#[derive(Debug, Clone)]
pub struct DockWidgetState<K> {
    /// The underlying split/tab layout graph.
    pub layout: DockLayout<K>,
    /// String-id lookup index, rebuilt automatically when the layout changes.
    pub index: DockIndex,
    /// Active tab-drag session, if a drag is in progress.
    pub drag: Option<DragSession>,
    /// Per-frame pane content drop targets (rebuilt each layout pass).
    pub drop_targets: Vec<(NodeId, Rectangle)>,
    /// Per-frame tab-bar drop targets (rebuilt each layout pass).
    pub tab_bar_targets: Vec<TabBarTarget>,
    /// Absolute bounds of each visible pane, collected each draw pass.
    pub pane_bounds: Vec<(NodeId, Rectangle)>,
    /// Pane that last received user focus (tab click or content click).
    pub focused_pane: Option<NodeId>,
    /// Pane that currently draws the focus frame.
    ///
    /// Sticky: only follows [`Self::focused_pane`] when the newly focused pane is eligible
    /// under [`Self::focus_frame_groups`]. Equal to `focused_pane` when no filter is set.
    pub focus_frame_pane: Option<NodeId>,
    /// Tab groups eligible for the focus frame. `None` means every pane is eligible.
    ///
    /// Set through [`DockBuilder::focus_frame_groups`](crate::widget::DockBuilder::focus_frame_groups).
    pub focus_frame_groups: Option<HashSet<String>>,
    /// Set when focus changed without a layout rebuild; triggers a redraw.
    pub focus_dirty: bool,
    /// Set when the layout tree changes and the cached widget root must rebuild.
    pub layout_dirty: bool,
}

impl<K> DockWidgetState<K> {
    /// Rebuild string-id index from the current layout graph.
    pub fn sync_index(&mut self) {
        self.index = DockIndex::rebuild_from_layout(&self.layout);
        self.resync_focus_frame();
    }

    /// Whether `pane` may draw the focus frame under the current group filter.
    #[must_use]
    pub fn frame_eligible(&self, pane: NodeId) -> bool {
        matches!(
            self.layout.kind(pane),
            Some(NodeKind::Pane(p))
                if pane_frame_eligible(self.focus_frame_groups.as_ref(), p)
        )
    }

    /// Move logical focus to `pane`, dragging the focus frame along when eligible.
    ///
    /// Returns `true` when either field changed.
    pub(crate) fn focus(&mut self, pane: NodeId) -> bool {
        let mut changed = false;
        if self.focused_pane != Some(pane) {
            self.focused_pane = Some(pane);
            changed = true;
        }
        if self.focus_frame_pane != Some(pane) && self.frame_eligible(pane) {
            self.focus_frame_pane = Some(pane);
            changed = true;
        }
        if changed {
            self.focus_dirty = true;
        }
        changed
    }

    /// Restrict the focus frame to panes tagged with one of `groups`.
    ///
    /// `None` restores the default, where every pane shows the frame while focused.
    pub fn set_focus_frame_groups(&mut self, groups: Option<HashSet<String>>) {
        if self.focus_frame_groups == groups {
            return;
        }
        self.focus_frame_groups = groups;
        self.resync_focus_frame();
        self.focus_dirty = true;
    }

    /// Re-resolve [`Self::focus_frame_pane`] after a policy or structural change.
    ///
    /// Repairs a frame that became dangling or ineligible; never resurrects one that was
    /// deliberately cleared.
    fn resync_focus_frame(&mut self) {
        if self.focused_pane.is_some_and(|p| self.frame_eligible(p)) {
            self.focus_frame_pane = self.focused_pane;
            return;
        }
        let Some(current) = self.focus_frame_pane else {
            return;
        };
        if self.frame_eligible(current) {
            return;
        }
        let groups = self.focus_frame_groups.as_ref();
        self.focus_frame_pane =
            first_pane_where(&self.layout, |_, pane| pane_frame_eligible(groups, pane));
    }

    pub(crate) fn commit_layout(&mut self) {
        if self.layout_dirty {
            self.sync_index();
            self.layout_dirty = false;
        }
    }

    /// Build widget state from a declarative [`LayoutTree`](crate::LayoutTree).
    pub fn from_tree(tree: crate::LayoutTree<K>) -> crate::Result<Self>
    where
        K: Copy,
    {
        let built = build_tree(&tree)?;
        let focused_pane = first_pane(&built.layout);
        Ok(Self::from_built(built, focused_pane))
    }

    /// Build widget state from a compiled layout.
    #[must_use]
    pub fn from_built(built: BuiltLayout<K>, focused_pane: Option<NodeId>) -> Self {
        Self {
            layout: built.layout,
            index: built.index,
            drag: None,
            drop_targets: Vec::new(),
            tab_bar_targets: Vec::new(),
            pane_bounds: Vec::new(),
            focused_pane,
            focus_frame_pane: focused_pane,
            focus_frame_groups: None,
            focus_dirty: false,
            layout_dirty: true,
        }
    }
}

/// Whether a pane's group tag passes the focus-frame filter (`None` accepts every pane).
fn pane_frame_eligible(groups: Option<&HashSet<String>>, pane: &Pane) -> bool {
    groups.is_none_or(|groups| pane.group.as_deref().is_some_and(|g| groups.contains(g)))
}

impl<K> Default for DockWidgetState<K> {
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
            focus_frame_pane: None,
            focus_frame_groups: None,
            focus_dirty: false,
            layout_dirty: false,
        }
    }
}

/// End an active drag at `cursor`, applying a drop when valid.
pub fn finish_drag<K>(state: &mut DockWidgetState<K>, cursor: Option<iced::Point>) -> bool {
    let Some(cursor) = cursor else {
        let had_drag = state.drag.is_some();
        state.drag = None;
        return had_drag;
    };

    let drop_targets = state.drop_targets.clone();
    let tab_bar_targets = state.tab_bar_targets.clone();
    let Some(mut session) = state.drag.take() else {
        return false;
    };

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
/// Does not emit [`DockEvent`](crate::DockEvent) values. After a successful structural change, call
/// [`DockWidgetState::sync_index`] or rely on the widget's next layout pass.
pub fn dispatch_action<K>(state: &mut DockWidgetState<K>, action: DockAction) -> bool {
    let factory = Factory;
    let mut changed = false;

    match action {
        DockAction::Tab(tab_msg) => match tab_msg {
            TabAction::Select { pane, panel } => {
                factory.set_active_panel(&mut state.layout, pane, panel);
                state.focus(pane);
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
            if state.focus(pane) {
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
