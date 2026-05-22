//! High-level dock state for applications.

use std::rc::Rc;

use crate::factory::{default_ide_layout, Factory, FactoryResult};
use crate::manager::{DockManager, DragSession};
use crate::model::{ContentKey, DockOperation, Layout, NodeId};
use crate::widget::dock_surface::DockMessage;
use crate::widget::tab_dock::TabMessage;

/// Application-facing dock state.
pub struct DockState<Message> {
    pub layout: Layout,
    factory: Factory,
    manager: DockManager,
    drag: Option<DragSession>,
    map: Rc<dyn Fn(DockMessage) -> Message>,
}

impl<Message: Clone + 'static> DockState<Message> {
    pub fn new(map: impl Fn(DockMessage) -> Message + 'static) -> Self {
        Self {
            layout: Layout::new(),
            factory: Factory::new(),
            manager: DockManager::new(),
            drag: None,
            map: Rc::new(map),
        }
    }

    pub fn factory(&self) -> &Factory {
        &self.factory
    }

    pub fn factory_mut(&mut self) -> &mut Factory {
        &mut self.factory
    }

    pub fn manager(&self) -> &DockManager {
        &self.manager
    }

    pub fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }

    /// Builds a default horizontal split: document tabs | tool tabs.
    pub fn init_default_layout(
        &mut self,
        documents: Vec<(String, ContentKey)>,
        tools: Vec<(String, ContentKey)>,
    ) -> FactoryResult<()> {
        default_ide_layout(&self.factory, &mut self.layout, documents, tools)
    }

    pub fn drag_session(&self) -> Option<&DragSession> {
        self.drag.as_ref()
    }

    pub fn on_message(&self) -> Rc<dyn Fn(DockMessage) -> Message> {
        self.map.clone()
    }

    pub fn update(&mut self, message: DockMessage) {
        match message {
            DockMessage::Tab(TabMessage::Selected { group, index }) => {
                if let Some(leaf) = tab_leaf_at(&self.layout, group, index) {
                    let _ = self.factory.set_active(&mut self.layout, group, leaf);
                }
            }
            DockMessage::Tab(TabMessage::Closed { group, index }) => {
                if let Some(leaf) = tab_leaf_at(&self.layout, group, index) {
                    let _ = self.factory.close(&mut self.layout, leaf);
                }
            }
            DockMessage::Tab(TabMessage::DragStarted { group, index, .. }) => {
                if let Some(leaf) = tab_leaf_at(&self.layout, group, index) {
                    self.drag = Some(DragSession {
                        source: leaf,
                        hover_target: None,
                        operation: DockOperation::Fill,
                    });
                }
            }
            DockMessage::Tab(TabMessage::DragMoved { .. }) => {}
            DockMessage::Tab(TabMessage::DragEnded {
                target,
                zone,
                ..
            }) => {
                if let (Some(mut session), Some(target)) = (self.drag.take(), target) {
                    session.hover_target = Some(target);
                    session.operation = zone.to_operation();
                    let _ = self
                        .manager
                        .execute(&self.factory, &mut self.layout, &session);
                }
            }
            DockMessage::SplitDrag { group, split_at } => {
                let _ = self
                    .factory
                    .set_binary_split_ratio(&mut self.layout, group, split_at);
            }
            DockMessage::LayoutChanged => {}
        }
    }

}

fn tab_leaf_at(layout: &Layout, group: NodeId, index: usize) -> Option<NodeId> {
    let crate::model::NodeKind::TabGroup(g) = layout.kind(group)? else {
        return None;
    };
    g.children.get(index).copied()
}
