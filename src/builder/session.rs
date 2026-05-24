use std::cell::RefCell;
use std::rc::Rc;

use crate::builder::compile::{
    active_panel_in_pane, build_tree, first_pane, insert_panel_runtime, owning_pane,
};
use crate::builder::index::DockIndex;
use crate::builder::spec::{LayoutTree, PanelDef};
use crate::factory::Factory;
use crate::model::{NodeId, NodeKind};
use crate::widget::{handle_dock_message, DockMessage, DockWidgetState};
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

/// High-level handle for a dock layout and runtime panel operations.
pub struct DockSession {
    inner: Rc<RefCell<DockWidgetState>>,
    index: RefCell<DockIndex>,
    index_stale: RefCell<bool>,
}

impl DockSession {
    /// Build a session from a declarative layout tree.
    pub fn from_tree(tree: LayoutTree) -> Result<Self> {
        let built = build_tree(&tree)?;
        let focused_pane = first_pane(&built.layout);
        let state = DockWidgetState {
            layout: built.layout,
            drag: None,
            drop_targets: Vec::new(),
            tab_bar_targets: Vec::new(),
            pane_bounds: Vec::new(),
            focused_pane,
            focus_dirty: false,
            layout_dirty: true,
        };
        Ok(Self {
            inner: Rc::new(RefCell::new(state)),
            index: RefCell::new(built.index),
            index_stale: RefCell::new(false),
        })
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

    /// Focus a panel by its string id.
    pub fn focus_panel(&self, panel_id: &str) -> Result {
        self.ensure_index_fresh();
        let panel_node = self
            .index
            .borrow()
            .panel_node(panel_id)
            .ok_or_else(|| Error::UnknownPanel(panel_id.into()))?;
        let pane_id =
            owning_pane(&self.inner.borrow().layout, panel_node).ok_or(Error::InvalidTarget)?;
        Factory.set_active_panel(&mut self.inner.borrow_mut().layout, pane_id, panel_node);
        self.inner.borrow_mut().focused_pane = Some(pane_id);
        self.inner.borrow_mut().focus_dirty = true;
        self.inner.borrow_mut().layout_dirty = true;
        Ok(())
    }

    /// Close a panel by its string id.
    pub fn close_panel(&self, panel_id: &str) -> Result {
        self.ensure_index_fresh();
        let panel_node = self
            .index
            .borrow()
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

    /// Pane that last received focus, if any.
    pub fn focused_pane(&self) -> Option<NodeId> {
        self.inner.borrow().focused_pane
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

    /// Currently focused panel id (active tab in the focused pane), if any.
    pub fn active_panel(&self) -> Option<String> {
        self.ensure_index_fresh();
        let state = self.inner.borrow();
        let pane = state.focused_pane?;
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

impl std::fmt::Debug for DockSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DockSession").finish_non_exhaustive()
    }
}
