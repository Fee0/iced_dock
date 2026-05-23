use std::cell::RefCell;
use std::rc::Rc;

use crate::builder::compile::{
    build_tree, first_pane, insert_panel_runtime, owning_pane, pane_for_active_panel,
};
use crate::builder::error::LayoutError;
use crate::builder::index::DockIndex;
use crate::builder::spec::{LayoutTree, PanelDef};
use crate::factory::Factory;
use crate::model::{Layout, NodeId};
use crate::widget::{handle_dock_message, DockMessage, DockWidgetState};

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
    last_active_pane: RefCell<Option<NodeId>>,
    index_stale: RefCell<bool>,
}

impl DockSession {
    /// Build a session from a declarative layout tree.
    pub fn from_tree(tree: LayoutTree) -> Result<Self, LayoutError> {
        let built = build_tree(&tree)?;
        let state = DockWidgetState {
            layout: built.layout,
            drag: None,
            drop_targets: Vec::new(),
            layout_dirty: true,
        };
        let last_active_pane = built
            .index
            .panels
            .values()
            .find_map(|&panel| pane_for_active_panel(&state.layout, panel));
        Ok(Self {
            inner: Rc::new(RefCell::new(state)),
            index: RefCell::new(built.index),
            last_active_pane: RefCell::new(last_active_pane),
            index_stale: RefCell::new(false),
        })
    }

    /// Shared widget state for the iced dock builder.
    pub fn state(&self) -> Rc<RefCell<DockWidgetState>> {
        self.inner.clone()
    }

    /// Apply a dock message and refresh internal indexes when the layout changes.
    pub fn apply_message(&self, msg: DockMessage) -> bool {
        let selected_pane = match &msg {
            DockMessage::Tab(crate::widget::TabMessage::Select { pane, .. }) => Some(*pane),
            _ => None,
        };
        let changed = {
            let mut state = self.inner.borrow_mut();
            handle_dock_message(&mut state, msg)
        };
        if changed {
            if self.inner.borrow().layout_dirty {
                *self.index_stale.borrow_mut() = true;
            }
            if let Some(pane) = selected_pane {
                *self.last_active_pane.borrow_mut() = Some(pane);
            }
        }
        changed
    }

    /// Open a panel in the given pane target and activate it.
    pub fn open_panel(
        &self,
        target: PaneTarget,
        panel: impl Into<PanelDef>,
    ) -> Result<(), LayoutError> {
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
            factory
                .add_panel_to_pane(&mut state.layout, pane_id, panel_id)
                .map_err(|_| LayoutError::OperationFailed("add_panel_to_pane"))?;
            factory.set_active_panel(&mut state.layout, pane_id, panel_id);
            state.layout_dirty = true;
        }
        *self.last_active_pane.borrow_mut() = Some(pane_id);
        Ok(())
    }

    /// Focus a panel by its string id.
    pub fn focus_panel(&self, panel_id: &str) -> Result<(), LayoutError> {
        self.ensure_index_fresh();
        let panel_node = self
            .index
            .borrow()
            .panel_node(panel_id)
            .ok_or_else(|| LayoutError::UnknownPanel(panel_id.into()))?;
        let pane_id = owning_pane(&self.inner.borrow().layout, panel_node)
            .ok_or(LayoutError::InvalidTarget)?;
        Factory.set_active_panel(&mut self.inner.borrow_mut().layout, pane_id, panel_node);
        *self.last_active_pane.borrow_mut() = Some(pane_id);
        self.inner.borrow_mut().layout_dirty = true;
        Ok(())
    }

    /// Close a panel by its string id.
    pub fn close_panel(&self, panel_id: &str) -> Result<(), LayoutError> {
        self.ensure_index_fresh();
        let panel_node = self
            .index
            .borrow()
            .panel_node(panel_id)
            .ok_or_else(|| LayoutError::UnknownPanel(panel_id.into()))?;
        let mut state = self.inner.borrow_mut();
        Factory
            .close(&mut state.layout, panel_node)
            .map_err(|_| LayoutError::OperationFailed("close"))?;
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

    /// Currently focused panel id, if any.
    pub fn active_panel(&self) -> Option<String> {
        self.ensure_index_fresh();
        active_panel_id(&self.inner.borrow().layout, &self.index.borrow())
    }

    fn ensure_index_fresh(&self) {
        if *self.index_stale.borrow() || self.inner.borrow().layout_dirty {
            let layout = &self.inner.borrow().layout;
            *self.index.borrow_mut() = DockIndex::rebuild_from_layout(layout);
            *self.index_stale.borrow_mut() = false;
        }
    }

    fn resolve_pane(&self, target: PaneTarget) -> Result<NodeId, LayoutError> {
        let layout = &self.inner.borrow().layout;
        match target {
            PaneTarget::Named(name) => self
                .index
                .borrow()
                .pane_node(name)
                .ok_or_else(|| LayoutError::UnknownPane(name.into())),
            PaneTarget::Active => (*self.last_active_pane.borrow())
                .or_else(|| {
                    self.active_panel().and_then(|id| {
                        self.index
                            .borrow()
                            .panel_node(&id)
                            .and_then(|p| owning_pane(layout, p))
                    })
                })
                .ok_or(LayoutError::InvalidTarget),
            PaneTarget::First => first_pane(layout).ok_or(LayoutError::InvalidTarget),
        }
    }
}

fn active_panel_id(layout: &Layout, index: &DockIndex) -> Option<String> {
    crate::builder::compile::active_panel_id(layout, index)
}

impl std::fmt::Debug for DockSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DockSession").finish_non_exhaustive()
    }
}
