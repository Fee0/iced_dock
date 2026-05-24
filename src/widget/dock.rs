use std::cell::RefCell;
use std::rc::Rc;

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::tree::{State, Tag, Tree};
use iced::advanced::widget::{Operation, Widget};
use iced::advanced::{Clipboard, Renderer as AdvRenderer, Shell};
use iced::mouse::{self, Cursor};
use iced::{Element, Event, Length, Rectangle, Size, Theme};

use crate::builder::DockIndex;
use crate::factory::Factory;
use crate::manager::{DockManager, DragSession, TabBarTarget};
use crate::model::{ContentKey, Layout as DockLayout, NodeId, NodeKind, Pane};
use crate::style::DockStyle;
use crate::widget::action::{DockAction, TabAction};
use crate::widget::event::{action_to_event, DockEvent};
use crate::widget::split::SplitContainer;
use crate::widget::tab_dock::{TabDock, TabInfo};

/// Persistent docking state (stored in the widget [`Tree`]).
#[derive(Debug, Clone)]
pub struct DockWidgetState {
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
}

impl DockWidgetState {
    /// Rebuild string-id index from the current layout graph.
    pub fn sync_index(&mut self) {
        self.index = DockIndex::rebuild_from_layout(&self.layout);
    }

    fn commit_layout(&mut self) {
        if self.layout_dirty {
            self.sync_index();
            self.layout_dirty = false;
        }
    }

    /// Build widget state from a declarative [`LayoutTree`](crate::LayoutTree).
    pub fn from_tree(tree: crate::LayoutTree) -> crate::Result<Self> {
        let built = crate::builder::compile::build_tree(&tree)?;
        let focused_pane = crate::builder::compile::first_pane(&built.layout);
        Ok(Self::from_built(built, focused_pane))
    }

    /// Build widget state from a compiled layout.
    pub fn from_built(
        built: crate::builder::compile::BuiltLayout,
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
        }
    }
}

impl Default for DockWidgetState {
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

    let drop_targets = state.drop_targets.clone();
    let tab_bar_targets = state.tab_bar_targets.clone();
    let Some(session) = state.drag.take() else {
        return false;
    };

    let mut session = session;
    DockManager::update_drag_hover_full(
        &mut session,
        cursor,
        &drop_targets,
        &tab_bar_targets,
    );
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

/// Tree state: layout data + cached root element (must match `tree.children`).
struct DockTreeHolder<Message> {
    dock_state: Rc<RefCell<DockWidgetState>>,
    root: RefCell<Option<Element<'static, Message, Theme, iced::Renderer>>>,
}

pub struct Dock<Message> {
    content: Rc<dyn Fn(ContentKey) -> Element<'static, Message, Theme, iced::Renderer>>,
    on_event: Rc<dyn Fn(DockEvent) -> Message>,
    external_state: Option<Rc<RefCell<DockWidgetState>>>,
    style: Rc<dyn Fn(&Theme) -> DockStyle>,
    tab_bar_scrollbar_hide_delay: iced::time::Duration,
    tab_bar_show_scrollbar: bool,
}

impl<Message: Clone + 'static> Dock<Message> {
    pub fn new(
        content: Rc<dyn Fn(ContentKey) -> Element<'static, Message, Theme, iced::Renderer>>,
        on_event: Rc<dyn Fn(DockEvent) -> Message>,
    ) -> Self {
        Self {
            content,
            on_event,
            external_state: None,
            style: Rc::new(DockStyle::from_theme),
            tab_bar_scrollbar_hide_delay: iced::time::Duration::from_secs(1),
            tab_bar_show_scrollbar: true,
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

    /// Delay before the tab-bar scrollbar hides after the pointer leaves the tab bar.
    ///
    /// Default is one second.
    pub fn tab_bar_scrollbar_hide_delay(mut self, delay: iced::time::Duration) -> Self {
        self.tab_bar_scrollbar_hide_delay = delay;
        self
    }

    /// Whether overflowing tab bars show a horizontal scrollbar thumb.
    ///
    /// When `false`, tabs can still be scrolled with the mouse wheel (and Shift+wheel).
    /// Default is `true`.
    pub fn tab_bar_show_scrollbar(mut self, show: bool) -> Self {
        self.tab_bar_show_scrollbar = show;
        self
    }

    fn wrap_action(
        holder: &Rc<RefCell<DockWidgetState>>,
        on_event: &Rc<dyn Fn(DockEvent) -> Message>,
        action: DockAction,
    ) -> Message {
        let mut state = holder.borrow_mut();
        let event = action_to_event(&state.layout, &state.index, &action)
            .unwrap_or(DockEvent::LayoutChanged);
        dispatch_action(&mut state, action);
        (on_event)(event)
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
                    Rc::new(move |action: DockAction| Self::wrap_action(&h, &on_event, action));
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
        let content_key = match &entry.kind {
            NodeKind::Panel(m) => m.content,
            _ => return None,
        };
        let tabs: Vec<TabInfo> = pane
            .tabs
            .iter()
            .filter_map(|&id| {
                let e = layout.get(id)?;
                match &e.kind {
                    NodeKind::Panel(m) => Some(TabInfo {
                        id,
                        title: m.title.clone(),
                        can_close: m.can_close,
                        can_drag: m.can_drag,
                    }),
                    _ => None,
                }
            })
            .collect();

        let content = (self.content)(content_key);
        let h = holder.clone();
        let on_event = self.on_event.clone();
        let on_tab = Rc::new(move |action: DockAction| Self::wrap_action(&h, &on_event, action));
        Some(
            TabDock::new(
                holder.clone(),
                pane_id,
                tabs,
                active,
                content,
                on_tab,
                self.style.clone(),
                self.tab_bar_scrollbar_hide_delay,
                self.tab_bar_show_scrollbar,
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

}

pub struct DockBuilder<Message> {
    content: Option<Rc<dyn Fn(ContentKey) -> Element<'static, Message, Theme, iced::Renderer>>>,
    on_event: Option<Rc<dyn Fn(DockEvent) -> Message>>,
    shared_state: Option<Rc<RefCell<DockWidgetState>>>,
    style: Option<Rc<dyn Fn(&Theme) -> DockStyle>>,
    min_pane_width: Option<f32>,
    min_pane_height: Option<f32>,
    tab_bar_scrollbar_hide_delay: Option<iced::time::Duration>,
    tab_bar_show_scrollbar: Option<bool>,
}

impl<Message> Default for DockBuilder<Message> {
    fn default() -> Self {
        Self {
            content: None,
            on_event: None,
            shared_state: None,
            style: None,
            min_pane_width: None,
            min_pane_height: None,
            tab_bar_scrollbar_hide_delay: None,
            tab_bar_show_scrollbar: None,
        }
    }
}

impl<Message: Clone + 'static> DockBuilder<Message> {
    pub fn content(
        mut self,
        f: impl Fn(ContentKey) -> Element<'static, Message, Theme, iced::Renderer> + 'static,
    ) -> Self {
        self.content = Some(Rc::new(f));
        self
    }

    /// Map observation [`DockEvent`] values to the application message type.
    ///
    /// The widget applies layout mutations before this callback; do not call
    /// [`DockSession::dispatch`](crate::DockSession::dispatch) for widget-originated events.
    pub fn on_event(mut self, f: impl Fn(DockEvent) -> Message + 'static) -> Self {
        self.on_event = Some(Rc::new(f));
        self
    }

    pub fn state(mut self, state: Rc<RefCell<DockWidgetState>>) -> Self {
        self.shared_state = Some(state);
        self
    }

    pub fn style(mut self, style: impl Fn(&Theme) -> DockStyle + 'static) -> Self {
        self.style = Some(Rc::new(style));
        self
    }

    /// Minimum width of each pane in horizontal split groups.
    ///
    /// Overrides [`SplitterStyle::min_pane_width`] on the resolved [`DockStyle`].
    /// Default is `80.0` from [`DockStyle::modern_dark`].
    pub fn min_pane_width(mut self, min_pane_width: f32) -> Self {
        self.min_pane_width = Some(min_pane_width.max(1.0));
        self
    }

    /// Minimum height of each pane in vertical split groups.
    ///
    /// Overrides [`SplitterStyle::min_pane_height`] on the resolved [`DockStyle`].
    /// Default is `80.0` from [`DockStyle::modern_dark`].
    pub fn min_pane_height(mut self, min_pane_height: f32) -> Self {
        self.min_pane_height = Some(min_pane_height.max(1.0));
        self
    }

    /// Delay before the tab-bar scrollbar hides after the pointer leaves the tab bar.
    ///
    /// Default is one second.
    pub fn tab_bar_scrollbar_hide_delay(
        mut self,
        delay: iced::time::Duration,
    ) -> Self {
        self.tab_bar_scrollbar_hide_delay = Some(delay);
        self
    }

    /// Whether overflowing tab bars show a horizontal scrollbar thumb.
    ///
    /// When `false`, tabs can still be scrolled with the mouse wheel (and Shift+wheel).
    /// Default is `true`.
    pub fn tab_bar_show_scrollbar(mut self, show: bool) -> Self {
        self.tab_bar_show_scrollbar = Some(show);
        self
    }

    pub fn build(self) -> Dock<Message> {
        let content = self.content.unwrap_or_else(|| {
            Rc::new(|_| iced::widget::text("No content").into())
                as Rc<dyn Fn(ContentKey) -> Element<'static, Message, Theme, iced::Renderer>>
        });
        let on_event = self
            .on_event
            .unwrap_or_else(|| Rc::new(|_| panic!("dock().on_event(...) required")));
        let mut dock = Dock::new(content, on_event);
        dock.external_state = self.shared_state;
        let base_style = self
            .style
            .unwrap_or_else(|| Rc::new(DockStyle::from_theme) as Rc<dyn Fn(&Theme) -> DockStyle>);
        let min_pane_width = self.min_pane_width;
        let min_pane_height = self.min_pane_height;
        if min_pane_width.is_some() || min_pane_height.is_some() {
            dock.style = Rc::new(move |theme| {
                let mut style = base_style(theme);
                if let Some(width) = min_pane_width {
                    style = style.with_min_pane_width(width);
                }
                if let Some(height) = min_pane_height {
                    style = style.with_min_pane_height(height);
                }
                style
            });
        } else {
            dock.style = base_style;
        }
        if let Some(delay) = self.tab_bar_scrollbar_hide_delay {
            dock.tab_bar_scrollbar_hide_delay = delay;
        }
        if let Some(show) = self.tab_bar_show_scrollbar {
            dock.tab_bar_show_scrollbar = show;
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
                .commit_layout();
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
        dock_state.borrow_mut().tab_bar_targets.clear();

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

        tree.state
            .downcast_ref::<DockTreeHolder<Message>>()
            .dock_state
            .borrow_mut()
            .pane_bounds
            .clear();

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

        if dock_state.borrow().layout_dirty {
            dock_state.borrow_mut().commit_layout();
            dock_state.borrow_mut().pane_bounds.clear();
            self.sync_root(tree);
        }

        if dock_state.borrow().focus_dirty {
            dock_state.borrow_mut().focus_dirty = false;
            shell.request_redraw();
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

/// Apply a [`DockAction`] to dock state (programmatic / session API).
///
/// Does not emit [`DockEvent`] values. After a successful structural change, call
/// [`DockWidgetState::sync_index`] or rely on the widget's next layout pass.
pub fn dispatch_action(state: &mut DockWidgetState, action: DockAction) -> bool {
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
                    if let Some(id) = state.index.panels.iter().find_map(|(s, &n)| {
                        (n == panel).then(|| s.clone())
                    }) {
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
