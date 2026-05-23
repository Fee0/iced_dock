use std::cell::RefCell;
use std::rc::Rc;

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::tree::{State, Tag, Tree};
use iced::advanced::widget::{Operation, Widget};
use iced::advanced::{Clipboard, Renderer as AdvRenderer, Shell};
use iced::advanced::renderer;
use iced::mouse::{self, Cursor};
use iced::{Element, Event, Length, Rectangle, Size, Theme};

use crate::factory::Factory;
use crate::manager::{DockManager, DragSession};
use crate::model::{ContentKey, Layout as DockLayout, NodeId, NodeKind, Pane};
use crate::style::DockStyle;
use crate::widget::message::{DockMessage, TabMessage};
use crate::widget::split::SplitContainer;
use crate::widget::tab_dock::{TabDock, TabInfo};

/// Persistent docking state (stored in the widget [`Tree`]).
#[derive(Debug, Clone)]
pub struct DockWidgetState {
    pub layout: DockLayout,
    pub drag: Option<DragSession>,
    pub drop_targets: Vec<(NodeId, Rectangle)>,
    /// Set when the layout tree changes and the cached widget root must rebuild.
    pub layout_dirty: bool,
}

impl Default for DockWidgetState {
    fn default() -> Self {
        Self {
            layout: DockLayout::new(),
            drag: None,
            drop_targets: Vec::new(),
            layout_dirty: false,
        }
    }
}

/// End an active drag at `cursor`, applying a drop when valid.
pub fn finish_drag(state: &mut DockWidgetState, cursor: Option<iced::Point>) -> bool {
    let Some(cursor) = cursor else {
        let had_drag = state.drag.is_some();
        state.drag = None;
        return had_drag;
    };

    let targets = state.drop_targets.clone();
    let Some(session) = state.drag.take() else {
        return false;
    };

    let mut session = session;
    DockManager::update_drag_hover(&mut session, cursor, &targets);
    let mut changed = false;
    if DockManager.execute(&mut state.layout, session).is_ok() {
        state.layout_dirty = true;
        changed = true;
    }
    changed
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
    style: Rc<dyn Fn(&Theme) -> DockStyle>,
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
            style: Rc::new(DockStyle::from_theme),
        }
    }

    pub fn style(mut self, style: impl Fn(&Theme) -> DockStyle + 'static) -> Self {
        self.style = Rc::new(style);
        self
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
        node: NodeId,
    ) -> Option<Element<'static, Message, Theme, iced::Renderer>> {
        match layout.kind(node)? {
            NodeKind::Proportional(pg) => {
                let children: Vec<_> = pg
                    .children
                    .iter()
                    .filter_map(|&c| self.build_node(holder, layout, c))
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
                        self.style.clone(),
                    )
                    .into(),
                )
            }
            NodeKind::Pane(p) => self.build_pane(holder, layout, node, p),
            NodeKind::Panel(_) => None,
            NodeKind::Root(_) => layout
                .root_child()
                .and_then(|c| self.build_node(holder, layout, c)),
        }
    }

    fn build_pane(
        &self,
        holder: &Rc<RefCell<DockWidgetState>>,
        layout: &DockLayout,
        pane_id: NodeId,
        pane: &Pane,
    ) -> Option<Element<'static, Message, Theme, iced::Renderer>> {
        let active = pane.active.or_else(|| pane.tabs.first().copied())?;
        let entry = layout.get(active)?;
        let (title, can_close, can_drag, content_key) = match &entry.kind {
            NodeKind::Panel(m) => (m.title.clone(), m.can_close, m.can_drag, m.content),
            _ => return None,
        };
        let tabs: Vec<TabInfo> = pane
            .tabs
            .iter()
            .filter_map(|&id| {
                let e = layout.get(id)?;
                let title = match &e.kind {
                    NodeKind::Panel(m) => m.title.clone(),
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
                holder.clone(),
                pane_id,
                title,
                can_close,
                can_drag,
                tabs,
                active,
                content,
                on_tab,
                self.style.clone(),
            )
            .into(),
        )
    }

    fn build_root_child(
        &self,
        holder: &Rc<RefCell<DockWidgetState>>,
    ) -> Option<Element<'static, Message, Theme, iced::Renderer>> {
        let state = holder.borrow();
        let root = state.layout.root_child()?;
        self.build_node(holder, &state.layout, root)
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
    style: Option<Rc<dyn Fn(&Theme) -> DockStyle>>,
}

impl<Message> Default for DockBuilder<Message> {
    fn default() -> Self {
        Self {
            content: None,
            on_event: None,
            shared_state: None,
            drag_active: false,
            style: None,
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

    pub fn style(mut self, style: impl Fn(&Theme) -> DockStyle + 'static) -> Self {
        self.style = Some(Rc::new(style));
        self
    }

    pub fn build(self) -> Dock<Message> {
        let content = self.content.unwrap_or(|_| iced::widget::text("No content").into());
        let on_event = self
            .on_event
            .unwrap_or_else(|| Rc::new(|_| panic!("dock().on_event(...) required")));
        let mut dock = Dock::new(content, on_event).drag_active(self.drag_active);
        dock.external_state = self.shared_state;
        if let Some(style) = self.style {
            dock.style = style;
        }
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
        let dirty = tree
            .state
            .downcast_ref::<DockTreeHolder<Message>>()
            .dock_state
            .borrow()
            .layout_dirty;
        if dirty {
            tree.state
                .downcast_ref::<DockTreeHolder<Message>>()
                .dock_state
                .borrow_mut()
                .layout_dirty = false;
        }
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
        let dock_state = tree
            .state
            .downcast_ref::<DockTreeHolder<Message>>()
            .dock_state
            .clone();
        dock_state.borrow_mut().drop_targets.clear();

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
        let dock_style = (self.style)(theme);
        renderer.fill_quad(
            renderer::Quad {
                bounds: layout.bounds(),
                ..renderer::Quad::default()
            },
            dock_style.background.color,
        );

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
        let dock_state = tree
            .state
            .downcast_ref::<DockTreeHolder<Message>>()
            .dock_state
            .clone();

        let Some(child_layout) = layout.children().next() else {
            return;
        };
        let Some(child_tree) = tree.children.first_mut() else {
            return;
        };
        {
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

        if let Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) = event {
            if dock_state.borrow().drag.is_some() {
                let changed = finish_drag(&mut dock_state.borrow_mut(), cursor.position());
                if changed {
                    shell.invalidate_layout();
                    shell.invalidate_widgets();
                }
                shell.request_redraw();
            }
        }

        if let Event::Mouse(mouse::Event::CursorMoved { .. }) = event {
            if dock_state.borrow().drag.is_some() {
                if let Some(pos) = cursor.position() {
                    let targets = dock_state.borrow().drop_targets.clone();
                    if let Some(ref mut session) = dock_state.borrow_mut().drag {
                        DockManager::update_drag_hover(session, pos, &targets);
                    }
                    shell.request_redraw();
                }
            }
        }

        if dock_state.borrow().layout_dirty {
            dock_state.borrow_mut().layout_dirty = false;
            self.sync_root(tree);
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
        let holder = tree.state.downcast_ref::<DockTreeHolder<Message>>();
        if holder.dock_state.borrow().drag.is_some() {
            return mouse::Interaction::Grab;
        }

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
    let mut changed = false;

    match msg {
        DockMessage::Tab(tab_msg) => match tab_msg {
            TabMessage::Select { pane, panel } => {
                factory.set_active_panel(&mut state.layout, pane, panel);
                state.layout_dirty = true;
                changed = true;
            }
            TabMessage::Close { panel } => {
                if factory.close(&mut state.layout, panel).is_ok() {
                    state.layout_dirty = true;
                    changed = true;
                }
            }
            TabMessage::DragStarted {
                source_pane,
                source_panel,
            } => {
                state.drag = Some(DragSession::new(source_pane, source_panel));
                changed = true;
            }
            TabMessage::DragEnded { cursor } => {
                if finish_drag(state, Some(cursor)) {
                    changed = true;
                }
            }
            TabMessage::DragMoved { cursor } => {
                let targets = state.drop_targets.clone();
                if let Some(ref mut session) = state.drag {
                    DockManager::update_drag_hover(session, cursor, &targets);
                }
            }
            TabMessage::DragCancelled => {
                state.drag = None;
                state.layout_dirty = true;
                changed = true;
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
                state.layout_dirty = true;
                changed = true;
            }
        }
        DockMessage::LayoutChanged => {}
    }
    changed
}
