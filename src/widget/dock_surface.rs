//! Root dock surface widget.

use std::rc::Rc;

use iced::Element;

use crate::manager::{DockManager, DragSession};
use crate::model::{ContentKey, Layout, NodeId};
use crate::view::tree::build_layout;
use crate::widget::tab_dock::TabMessage;

/// Messages emitted by the dock surface.
#[derive(Debug, Clone)]
pub enum DockMessage {
    Tab(TabMessage),
    SplitDrag { group: NodeId, split_at: f32 },
    LayoutChanged,
}

/// Build the full dock UI from a layout tree.
pub struct DockSurface<'a, Message: 'static> {
    layout: &'a Layout,
    drag: Option<&'a DragSession>,
    _manager: &'a DockManager,
    on_message: Rc<dyn Fn(DockMessage) -> Message>,
    content: &'static dyn Fn(ContentKey) -> Element<'static, Message>,
}

impl<'a, Message: Clone + 'static> DockSurface<'a, Message> {
    pub fn new(
        layout: &'a Layout,
        drag: Option<&'a DragSession>,
        manager: &'a DockManager,
        on_message: Rc<dyn Fn(DockMessage) -> Message>,
        content: &'static dyn Fn(ContentKey) -> Element<'static, Message>,
    ) -> Self {
        Self {
            layout,
            drag,
            _manager: manager,
            on_message,
            content,
        }
    }

    pub fn view(&self) -> Element<'a, Message> {
        let root_child = match self.layout.root_child() {
            Some(c) => c,
            None => return iced::widget::text("empty dock").into(),
        };

        build_layout(
            self.layout,
            self.drag,
            self.on_message.clone(),
            self.content,
            root_child,
        )
    }
}

impl<'a, Message: Clone + 'static> From<DockSurface<'a, Message>> for Element<'a, Message> {
    fn from(surface: DockSurface<'a, Message>) -> Self {
        surface.view()
    }
}
