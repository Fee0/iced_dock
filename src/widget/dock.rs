use std::cell::RefCell;
use std::rc::Rc;

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::tree::{State, Tag, Tree};
use iced::advanced::widget::{Operation, Widget};
use iced::advanced::{Clipboard, Shell};
use iced::mouse::{self, Cursor};
use iced::{Element, Event, Length, Rectangle, Size, Theme};

use crate::factory::Factory;
use crate::manager::{DockManager, DragSession};
use crate::model::{ContentKey, Layout as DockLayout, NodeId, NodeKind, TabGroup};
use crate::widget::message::{DockMessage, TabMessage};
use crate::widget::split::SplitContainer;
use crate::widget::tab_dock::{TabDock, TabInfo};

/// Persistent docking state (stored in the widget [`Tree`]).
#[derive(Debug, Clone)]
pub struct DockWidgetState {
    pub layout: DockLayout,
    pub drag: Option<DragSession>,
}

impl Default for DockWidgetState {
    fn default() -> Self {
        let mut layout = DockLayout::new();
        Factory
            .complex_ide_layout(&mut layout)
            .expect("complex_ide_layout seed");
        Self {
            layout,
            drag: None,
        }
    }
}

/// Tree state: layout data + cached root element (must match `tree.children`).
struct DockTreeHolder<Message> {
    dock_state: Rc<RefCell<DockWidgetState>>,
    root: RefCell<Option<Element<'static, Message, Theme, iced::Renderer>>>,
}

pub struct Dock<Message> {
    content: fn(ContentKey) -> Element<'static, Message, Theme, iced::Renderer>,
    on_event: Rc<dyn Fn(DockMessage) -> Message>,
    external_state: Option<Rc<RefCell<DockWidgetState>>>,
    drag_active: bool,
}

impl<Message: Clone + 'static> Dock<Message> {
    pub fn new(
        content: fn(ContentKey) -> Element<'static, Message, Theme, iced::Renderer>,
        on_event: Rc<dyn Fn(DockMessage) -> Message>,
    ) -> Self {
        Self {
            content,
            on_event,
            external_state: None,
            drag_active: false,
        }
    }

    pub fn with_state(mut self, state: Rc<RefCell<DockWidgetState>>) -> Self {
        self.external_state = Some(state);
        self
    }

    pub fn drag_active(mut self, active: bool) -> Self {
        self.drag_active = active;
        self
    }

    fn wrap_event(
        holder: &Rc<RefCell<DockWidgetState>>,
        on_event: &Rc<dyn Fn(DockMessage) -> Message>,
        msg: DockMessage,
    ) -> Message {
        let _ = handle_dock_message_impl(&mut holder.borrow_mut(), msg.clone());
        (on_event)(msg)
    }

    fn build_node(
        &self,
        holder: &Rc<RefCell<DockWidgetState>>,
        layout: &DockLayout,
        global_drag_active: bool,
        node: NodeId,
    ) -> Option<Element<'static, Message, Theme, iced::Renderer>> {
        match layout.kind(node)? {
            NodeKind::Proportional(pg) => {
                let children: Vec<_> = pg
                    .children
                    .iter()
                    .filter_map(|&c| self.build_node(holder, layout, global_drag_active, c))
                    .collect();
                if children.is_empty() {
                    return None;
                }
                let h = holder.clone();
                let on_event = self.on_event.clone();
                let on_split =
                    Rc::new(move |msg: DockMessage| Self::wrap_event(&h, &on_event, msg));
                Some(
                    SplitContainer::new(
                        node,
                        pg.axis,
                        pg.proportions.clone(),
                        children,
                        on_split,
                    )
                    .into(),
                )
            }
            NodeKind::TabGroup(g) => {
                self.build_tab_group(holder, layout, global_drag_active, node, g)
            }
            NodeKind::Document(_) | NodeKind::Tool(_) => None,
            NodeKind::Root(_) => layout
                .root_child()
                .and_then(|c| self.build_node(holder, layout, global_drag_active, c)),
        }
    }

    fn build_tab_group(
        &self,
        holder: &Rc<RefCell<DockWidgetState>>,
        layout: &DockLayout,
        global_drag_active: bool,
        group_id: NodeId,
        group: &TabGroup,
    ) -> Option<Element<'static, Message, Theme, iced::Renderer>> {
        let active = group.active.or_else(|| group.children.first().copied())?;
        let entry = layout.get(active)?;
        let (title, can_close, can_drag, content_key) = match &entry.kind {
            NodeKind::Document(m) | NodeKind::Tool(m) => {
                (m.title.clone(), m.can_close, m.can_drag, m.content)
            }
            _ => return None,
        };
        let tabs: Vec<TabInfo> = group
            .children
            .iter()
            .filter_map(|&id| {
                let e = layout.get(id)?;
                let title = match &e.kind {
                    NodeKind::Document(m) | NodeKind::Tool(m) => m.title.clone(),
                    _ => return None,
                };
                Some(TabInfo { id, title })
            })
            .collect();

        let content = (self.content)(content_key);
        let h = holder.clone();
        let on_event = self.on_event.clone();
        let on_tab = Rc::new(move |msg: DockMessage| Self::wrap_event(&h, &on_event, msg));
        Some(
            TabDock::new(
                group_id,
                title,
                can_close,
                can_drag,
                tabs,
                active,
                content,
                global_drag_active,
                on_tab,
            )
            .into(),
        )
    }

    fn build_root_child(
        &self,
        holder: &Rc<RefCell<DockWidgetState>>,
    ) -> Option<Element<'static, Message, Theme, iced::Renderer>> {
        let state = holder.borrow();
        let global_drag_active = self.drag_active || state.drag.is_some();
        let root = state.layout.root_child()?;
        self.build_node(holder, &state.layout, global_drag_active, root)
    }

    /// Rebuild cached root element and reconcile `tree.children`.
    fn sync_root(&self, tree: &mut Tree) {
        let dock_state = tree
            .state
            .downcast_ref::<DockTreeHolder<Message>>()
            .dock_state
            .clone();
        let new_root = self.build_root_child(&dock_state);
        {
            let holder = tree.state.downcast_mut::<DockTreeHolder<Message>>();
            holder.root.replace(new_root);
        }
        if let Some(child) = tree
            .state
            .downcast_ref::<DockTreeHolder<Message>>()
            .root
            .borrow()
            .as_ref()
        {
            if tree.children.is_empty() {
                tree.children.push(Tree::new(child));
            } else {
                tree.children[0].diff(child);
            }
        } else {
            tree.children.clear();
        }
    }

    fn with_cached_root<R>(
        tree: &Tree,
        f: impl FnOnce(&Element<'static, Message, Theme, iced::Renderer>) -> R,
    ) -> Option<R> {
        let holder = tree.state.downcast_ref::<DockTreeHolder<Message>>();
        holder.root.borrow().as_ref().map(f)
    }

    pub fn handle_message(state: &mut DockWidgetState, msg: DockMessage) -> bool {
        handle_dock_message_impl(state, msg)
    }
}

pub struct DockBuilder<Message> {
    content: Option<fn(ContentKey) -> Element<'static, Message, Theme, iced::Renderer>>,
    on_event: Option<Rc<dyn Fn(DockMessage) -> Message>>,
    shared_state: Option<Rc<RefCell<DockWidgetState>>>,
    drag_active: bool,
}

impl<Message> Default for DockBuilder<Message> {
    fn default() -> Self {
        Self {
            content: None,
            on_event: None,
            shared_state: None,
            drag_active: false,
        }
    }
}

impl<Message: Clone + 'static> DockBuilder<Message> {
    pub fn content(
        mut self,
        f: fn(ContentKey) -> Element<'static, Message, Theme, iced::Renderer>,
    ) -> Self {
        self.content = Some(f);
        self
    }

    pub fn on_event(mut self, f: impl Fn(DockMessage) -> Message + 'static) -> Self {
        self.on_event = Some(Rc::new(f));
        self
    }

    pub fn state(mut self, state: Rc<RefCell<DockWidgetState>>) -> Self {
        self.shared_state = Some(state);
        self
    }

    pub fn drag_active(mut self, active: bool) -> Self {
        self.drag_active = active;
        self
    }

    pub fn build(self) -> Dock<Message> {
        let content = self.content.unwrap_or(|_| iced::widget::text("No content").into());
        let on_event = self
            .on_event
            .unwrap_or_else(|| Rc::new(|_| panic!("dock().on_event(...) required")));
        let mut dock = Dock::new(content, on_event).drag_active(self.drag_active);
        dock.external_state = self.shared_state;
        dock
    }
}

pub fn dock<Message>() -> DockBuilder<Message>
where
    Message: Clone + 'static,
{
    DockBuilder::default()
}

impl<Message> Widget<Message, Theme, iced::Renderer> for Dock<Message>
where
    Message: Clone + 'static,
{
    fn tag(&self) -> Tag {
        Tag::of::<DockTreeHolder<Message>>()
    }

    fn state(&self) -> State {
        let dock_state = self
            .external_state
            .clone()
            .unwrap_or_else(|| Rc::new(RefCell::new(DockWidgetState::default())));
        State::new(DockTreeHolder::<Message> {
            dock_state,
            root: RefCell::new(None),
        })
    }

    fn diff(&self, tree: &mut Tree) {
        self.sync_root(tree);
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Fill,
            height: Length::Fill,
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        if tree.children.is_empty() {
            self.sync_root(tree);
        }
        let size = limits.max();
        if let Some(child_tree) = tree.children.first_mut() {
            let mut root = tree
                .state
                .downcast_mut::<DockTreeHolder<Message>>()
                .root
                .borrow_mut();
            if let Some(child) = root.as_mut() {
                let child_node = child.as_widget_mut().layout(child_tree, renderer, limits);
                return layout::Node::with_children(size, vec![child_node]);
            }
        }
        layout::Node::new(size)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        style: &iced::advanced::renderer::Style,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
    ) {
        let Some(child_layout) = layout.children().next() else {
            return;
        };
        let Some(child_tree) = tree.children.first() else {
            return;
        };
        let _ = Self::with_cached_root(tree, |child| {
            child.as_widget().draw(
                child_tree,
                renderer,
                theme,
                style,
                child_layout,
                cursor,
                viewport,
            );
        });
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let Some(child_layout) = layout.children().next() else {
            return;
        };
        let Some(child_tree) = tree.children.first_mut() else {
            return;
        };
        let mut root = tree
            .state
            .downcast_mut::<DockTreeHolder<Message>>()
            .root
            .borrow_mut();
        if let Some(child) = root.as_mut() {
            child.as_widget_mut().update(
                child_tree,
                event,
                child_layout,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let Some(child_layout) = layout.children().next() else {
            return mouse::Interaction::default();
        };
        let Some(child_tree) = tree.children.first() else {
            return mouse::Interaction::default();
        };
        Self::with_cached_root(tree, |child| {
            child.as_widget().mouse_interaction(
                child_tree,
                child_layout,
                cursor,
                viewport,
                renderer,
            )
        })
        .unwrap_or_default()
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        let Some(child_layout) = layout.children().next() else {
            return;
        };
        let Some(child_tree) = tree.children.first_mut() else {
            return;
        };
        let mut root = tree
            .state
            .downcast_mut::<DockTreeHolder<Message>>()
            .root
            .borrow_mut();
        if let Some(child) = root.as_mut() {
            child
                .as_widget_mut()
                .operate(child_tree, child_layout, renderer, operation);
        }
    }
}

impl<Message> From<Dock<Message>> for Element<'static, Message, Theme, iced::Renderer>
where
    Message: Clone + 'static,
{
    fn from(widget: Dock<Message>) -> Self {
        Element::new(widget)
    }
}

/// Apply a dock message to shared dock state.
pub fn apply_message(state: &RefCell<DockWidgetState>, msg: DockMessage) -> bool {
    handle_dock_message(&mut state.borrow_mut(), msg)
}

/// Apply layout mutations from a [`DockMessage`].
pub fn handle_dock_message(state: &mut DockWidgetState, msg: DockMessage) -> bool {
    handle_dock_message_impl(state, msg)
}

fn handle_dock_message_impl(state: &mut DockWidgetState, msg: DockMessage) -> bool {
    let factory = Factory;
    let manager = DockManager;
    let mut changed = false;

    match msg {
        DockMessage::Tab(tab_msg) => match tab_msg {
            TabMessage::Select { group, tab } => {
                factory.set_active_tab(&mut state.layout, group, tab);
                changed = true;
            }
            TabMessage::Close { tab } => {
                if factory.close(&mut state.layout, tab).is_ok() {
                    changed = true;
                }
            }
            TabMessage::DragStarted {
                source_group,
                source_tab,
            } => {
                state.drag = Some(DragSession::new(source_group, source_tab));
                changed = true;
            }
            TabMessage::DragMoved { target, zone } => {
                if let Some(ref mut session) = state.drag {
                    session.hover_target = Some(target);
                    session.operation = Some(zone.to_operation());
                }
            }
            TabMessage::DragEnded { target, zone } => {
                if let Some(session) = state.drag.take() {
                    let mut session = session;
                    session.hover_target = Some(target);
                    session.operation = Some(zone.to_operation());
                    if manager.execute(&mut state.layout, session).is_ok() {
                        changed = true;
                    }
                }
            }
        },
        DockMessage::SplitDrag {
            group,
            splitter_index,
            ratio,
        } => {
            if factory
                .adjust_splitter(&mut state.layout, group, splitter_index, ratio)
                .is_ok()
            {
                changed = true;
            }
        }
        DockMessage::LayoutChanged => {}
    }
    changed
}
