use std::cell::RefCell;
use std::rc::Rc;

use crate::builder::compile::{
    active_panel_in_pane, build_tree, first_pane, insert_panel_runtime, pane_for_panel, BuiltLayout,
};
use crate::builder::index::DockIndex;
use crate::builder::spec::{LayoutTree, PanelDef};
use crate::factory::Factory;
use crate::model::{NodeId, NodeKind};
use crate::spatial::{adjacent_pane, pane_bounds_map, Direction};
use crate::widget::{handle_dock_message, DockMessage, DockWidgetState, TabMessage};
use crate::{Error, Result};

/// Target pane for opening a new panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneTarget {
    /// Open in the pane registered with [`TabsNode::named`](crate::builder::TabsNode::named).
    Named(&'static str),
    /// Open in the pane that last received focus.
    Active,
    /// Open in the first pane encountered in a preorder tree walk.
    First,
}

/// Which pane receives focus when building a [`DockSession`].
#[derive(Debug, Clone, Copy, Default)]
pub enum InitialFocus<'a> {
    /// First pane in preorder tree walk.
    #[default]
    FirstPane,
    /// Pane registered with [`TabsNode::named`](crate::builder::TabsNode::named).
    NamedPane(&'a str),
    /// Pane that owns the panel with this id (active tab comes from `.active()` at compile time).
    NamedPanel(&'a str),
}

/// Direction to cycle the active tab within the focused pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelCycle {
    Next,
    Prev,
}

/// High-level handle for a dock layout and runtime panel operations.
pub struct DockSession {
    inner: Rc<RefCell<DockWidgetState>>,
    index: RefCell<DockIndex>,
    index_stale: RefCell<bool>,
}

impl DockSession {
    /// Build a session from a declarative layout tree.
    pub fn from_tree(tree: LayoutTree) -> Result<Self> {
        Self::from_tree_with_focus(tree, InitialFocus::default())
    }

    /// Build a session and set initial pane focus.
    pub fn from_tree_with_focus(tree: LayoutTree, focus: InitialFocus<'_>) -> Result<Self> {
        let built = build_tree(&tree)?;
        let focused_pane = resolve_initial_focus(&built, focus)?;
        Ok(Self::from_built(built, focused_pane))
    }

    /// Build a session from a compiled layout and index.
    pub fn from_built(built: BuiltLayout, focused_pane: Option<NodeId>) -> Self {
        let index = built.index.clone();
        let state = DockWidgetState::from_built(built, focused_pane);
        Self {
            inner: Rc::new(RefCell::new(state)),
            index: RefCell::new(index),
            index_stale: RefCell::new(false),
        }
    }

    /// Shared widget state for the iced dock builder.
    pub fn state(&self) -> Rc<RefCell<DockWidgetState>> {
        self.inner.clone()
    }

    /// Apply a dock message and refresh internal indexes when the layout changes.
    pub fn apply_message(&self, msg: DockMessage) -> bool {
        let changed = {
            let mut state = self.inner.borrow_mut();
            handle_dock_message(&mut state, msg)
        };
        if changed && self.inner.borrow().layout_dirty {
            *self.index_stale.borrow_mut() = true;
        }
        changed
    }

    /// Open a panel in the given pane target and activate it.
    pub fn open_panel(&self, target: PaneTarget, panel: impl Into<PanelDef>) -> Result {
        self.ensure_index_fresh();
        let def = panel.into();
        let pane_id = self.resolve_pane(target)?;
        let factory = Factory;
        let panel_id = {
            let mut state = self.inner.borrow_mut();
            let mut index = self.index.borrow_mut();
            insert_panel_runtime(&factory, &mut state.layout, &mut index, &def)?
        };
        {
            let mut state = self.inner.borrow_mut();
            factory.add_panel_to_pane(&mut state.layout, pane_id, panel_id)?;
            factory.set_active_panel(&mut state.layout, pane_id, panel_id);
            state.layout_dirty = true;
            state.focused_pane = Some(pane_id);
            state.focus_dirty = true;
        }
        Ok(())
    }

    /// Activate a panel by string id and focus its pane.
    pub fn select_panel(&self, panel_id: &str) -> Result {
        self.ensure_index_fresh();
        let panel_node = self
            .panel_node(panel_id)
            .ok_or_else(|| Error::UnknownPanel(panel_id.into()))?;
        let pane_id = self
            .pane_for_panel(panel_id)
            .ok_or(Error::InvalidTarget)?;
        self.apply_message(DockMessage::Tab(TabMessage::Select {
            pane: pane_id,
            panel: panel_node,
        }));
        Ok(())
    }

    /// Focus a panel by its string id (activates tab and focuses pane).
    pub fn focus_panel(&self, panel_id: &str) -> Result {
        self.select_panel(panel_id)
    }

    /// Close a panel by its string id.
    pub fn close_panel(&self, panel_id: &str) -> Result {
        self.ensure_index_fresh();
        let panel_node = self
            .panel_node(panel_id)
            .ok_or_else(|| Error::UnknownPanel(panel_id.into()))?;
        let mut state = self.inner.borrow_mut();
        Factory.close(&mut state.layout, panel_node)?;
        self.index.borrow_mut().panels.remove(panel_id);
        *self.index_stale.borrow_mut() = true;
        state.layout_dirty = true;
        Ok(())
    }

    /// All known panel ids.
    pub fn panel_ids(&self) -> Vec<String> {
        self.ensure_index_fresh();
        self.index.borrow().panel_ids().cloned().collect()
    }

    /// Panel node id for a string panel id.
    pub fn panel_node(&self, panel_id: &str) -> Option<NodeId> {
        self.ensure_index_fresh();
        self.index.borrow().panel_node(panel_id)
    }

    /// Pane that owns a panel identified by string id.
    pub fn pane_for_panel(&self, panel_id: &str) -> Option<NodeId> {
        self.ensure_index_fresh();
        let state = self.inner.borrow();
        pane_for_panel(&state.layout, &self.index.borrow(), panel_id)
    }

    /// Pane that last received focus, if any.
    pub fn focused_pane(&self) -> Option<NodeId> {
        self.inner.borrow().focused_pane
    }

    /// Whether the given pane currently has global focus.
    pub fn is_pane_focused(&self, pane: NodeId) -> bool {
        self.focused_pane() == Some(pane)
    }

    /// Focus a pane by id (does not change the active tab).
    pub fn focus_pane(&self, pane: NodeId) -> Result {
        if !matches!(self.inner.borrow().layout.kind(pane), Some(NodeKind::Pane(_))) {
            return Err(Error::InvalidTarget);
        }
        handle_dock_message(
            &mut self.inner.borrow_mut(),
            DockMessage::PaneFocused {
                pane,
                panel: None,
            },
        );
        Ok(())
    }

    /// Move focus to the nearest pane in `direction`.
    ///
    /// Requires at least one draw pass so [`DockWidgetState::pane_bounds`] is populated.
    /// Returns `true` if focus moved to a neighbor.
    pub fn focus_adjacent(&self, direction: Direction) -> bool {
        let Some(pane) = self.focused_pane() else {
            return false;
        };
        let bounds = pane_bounds_map(&self.inner.borrow().pane_bounds);
        let Some(adjacent) = adjacent_pane(pane, direction, &bounds) else {
            return false;
        };
        let _ = self.focus_pane(adjacent);
        true
    }

    /// Clear global pane focus without changing active tabs.
    pub fn clear_focus(&self) {
        let mut state = self.inner.borrow_mut();
        if state.focused_pane.is_some() {
            state.focused_pane = None;
            state.focus_dirty = true;
        }
    }

    /// Cycle the active tab in the focused pane (wraps at ends).
    pub fn cycle_panel(&self, cycle: PanelCycle) -> Result {
        let pane = self.focused_pane().ok_or(Error::InvalidTarget)?;
        let state = self.inner.borrow();
        let NodeKind::Pane(pane_state) = state.layout.kind(pane).ok_or(Error::InvalidTarget)?
        else {
            return Err(Error::InvalidTarget);
        };
        if pane_state.tabs.is_empty() {
            return Err(Error::InvalidTarget);
        }
        let current = pane_state
            .active
            .or(pane_state.tabs.first().copied())
            .ok_or(Error::InvalidTarget)?;
        let current_index = pane_state
            .tabs
            .iter()
            .position(|&id| id == current)
            .unwrap_or(0);
        let len = pane_state.tabs.len();
        let next_index = match cycle {
            PanelCycle::Next => (current_index + 1) % len,
            PanelCycle::Prev => (current_index + len - 1) % len,
        };
        let panel = pane_state.tabs[next_index];
        drop(state);
        self.apply_message(DockMessage::Tab(TabMessage::Select { pane, panel }));
        Ok(())
    }

    /// Currently focused panel id (active tab in the focused pane), if any.
    pub fn active_panel(&self) -> Option<String> {
        self.ensure_index_fresh();
        let state = self.inner.borrow();
        let pane = state.focused_pane?;
        active_panel_in_pane(&state.layout, &self.index.borrow(), pane)
    }

    /// Active panel id string in a specific pane (regardless of global focus).
    pub fn active_panel_in_pane(&self, pane: NodeId) -> Option<String> {
        self.ensure_index_fresh();
        let state = self.inner.borrow();
        active_panel_in_pane(&state.layout, &self.index.borrow(), pane)
    }

    fn ensure_index_fresh(&self) {
        if *self.index_stale.borrow() || self.inner.borrow().layout_dirty {
            let layout = &self.inner.borrow().layout;
            *self.index.borrow_mut() = DockIndex::rebuild_from_layout(layout);
            *self.index_stale.borrow_mut() = false;
        }
    }

    fn resolve_pane(&self, target: PaneTarget) -> Result<NodeId> {
        match target {
            PaneTarget::Named(name) => self
                .index
                .borrow()
                .pane_node(name)
                .ok_or_else(|| Error::UnknownPane(name.into())),
            PaneTarget::Active => self
                .inner
                .borrow()
                .focused_pane
                .ok_or(Error::InvalidTarget),
            PaneTarget::First => first_pane(&self.inner.borrow().layout).ok_or(Error::InvalidTarget),
        }
    }
}

fn resolve_initial_focus(built: &BuiltLayout, focus: InitialFocus<'_>) -> Result<Option<NodeId>> {
    match focus {
        InitialFocus::FirstPane => Ok(first_pane(&built.layout)),
        InitialFocus::NamedPane(name) => built
            .index
            .pane_node(name)
            .ok_or_else(|| Error::UnknownPane(name.into()))
            .map(Some),
        InitialFocus::NamedPanel(panel_id) => pane_for_panel(&built.layout, &built.index, panel_id)
            .ok_or_else(|| Error::UnknownPanel(panel_id.into()))
            .map(Some),
    }
}

impl std::fmt::Debug for DockSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DockSession").finish_non_exhaustive()
    }
}
