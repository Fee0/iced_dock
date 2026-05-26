use std::cell::RefCell;
use std::rc::Rc;

use iced::advanced;
use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::tree::{State, Tag, Tree};
use iced::advanced::widget::{Operation, Widget};
use iced::advanced::{Clipboard, Shell};
use iced::mouse::{self, Cursor};
use iced::time::Duration;
use iced::widget::overlay::menu;
use iced::widget::{self, button, container, text as iced_text};
use iced::{Element, Event, Length, Rectangle, Size, Vector};

use crate::model::{Layout as DockLayout, NodeId, NodeKind, Pane};
use crate::style::{Catalog, DockStyle, PaneContent, StyleFn};
use crate::widget::action::DockAction;
use crate::widget::event::{action_to_event, DockEvent};
use crate::widget::split::SplitContainer;
use crate::widget::state::{dispatch_action, DockWidgetState};
use crate::widget::tab_dock::{TabDock, TabInfo};

/// Tree state: layout data + cached root element (must match `tree.children`).
struct DockTreeHolder<K, Message, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: advanced::Renderer,
{
    dock_state: Rc<RefCell<DockWidgetState<K, Theme>>>,
    root: Option<Element<'static, Message, Theme, Renderer>>,
}

pub struct Dock<K, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Theme: Catalog,
    Renderer: advanced::Renderer,
{
    content: Rc<dyn Fn(K) -> PaneContent<'static, Message, Theme, Renderer>>,
    on_event: Rc<dyn Fn(DockEvent) -> Message>,
    external_state: Option<Rc<RefCell<DockWidgetState<K, Theme>>>>,
    class: Rc<<Theme as Catalog>::Class<'static>>,
    min_pane_width: f32,
    min_pane_height: f32,
    drag_threshold: f32,
    drop_edge_fraction: f32,
    tab_bar_scrollbar_hide_delay: Duration,
    tab_bar_show_scrollbar: bool,
}

impl<K, Message, Theme, Renderer> Dock<K, Message, Theme, Renderer>
where
    K: Copy + 'static,
    Message: Clone + 'static,
    Theme: Catalog
        + button::Catalog
        + container::Catalog
        + iced_text::Catalog
        + menu::Catalog
        + Clone
        + PartialEq
        + 'static,
    Renderer: advanced::Renderer + advanced::text::Renderer + 'static,
    <Theme as button::Catalog>::Class<'static>: From<button::StyleFn<'static, Theme>>,
    <Theme as container::Catalog>::Class<'static>: From<container::StyleFn<'static, Theme>>,
    for<'b> <Theme as iced_text::Catalog>::Class<'b>: From<iced_text::StyleFn<'b, Theme>>,
{
    pub fn new(
        content: Rc<dyn Fn(K) -> PaneContent<'static, Message, Theme, Renderer>>,
        on_event: Rc<dyn Fn(DockEvent) -> Message>,
    ) -> Self {
        Self {
            content,
            on_event,
            external_state: None,
            class: Rc::new(<Theme as Catalog>::default()),
            min_pane_width: 80.0,
            min_pane_height: 80.0,
            drag_threshold: 6.0,
            drop_edge_fraction: 0.2,
            tab_bar_scrollbar_hide_delay: Duration::from_secs(1),
            tab_bar_show_scrollbar: false,
        }
    }

    fn theme_cell(holder: &Rc<RefCell<DockWidgetState<K, Theme>>>) -> Rc<RefCell<Option<Theme>>> {
        Rc::clone(&holder.borrow().resolved_theme)
    }

    #[must_use]
    pub fn style(mut self, style: impl Fn(&Theme) -> DockStyle + 'static) -> Self
    where
        <Theme as Catalog>::Class<'static>: From<StyleFn<'static, Theme>>,
    {
        self.class = Rc::new((Box::new(style) as StyleFn<'static, Theme>).into());
        self
    }

    /// Sets the style class of the [`Dock`].
    #[must_use]
    pub fn class(mut self, class: <Theme as Catalog>::Class<'static>) -> Self {
        self.class = Rc::new(class);
        self
    }

    #[must_use]
    pub fn with_state(mut self, state: Rc<RefCell<DockWidgetState<K, Theme>>>) -> Self {
        self.external_state = Some(state);
        self
    }

    /// Delay before the tab-bar scrollbar hides after the pointer leaves the tab bar.
    ///
    /// Default is one second.
    #[must_use]
    pub fn tab_bar_scrollbar_hide_delay(mut self, delay: Duration) -> Self {
        self.tab_bar_scrollbar_hide_delay = delay;
        self
    }

    /// Whether overflowing tab bars show a horizontal scrollbar thumb.
    ///
    /// When `false`, tabs can still be scrolled with the mouse wheel (and Shift+wheel).
    /// Default is `true`.
    #[must_use]
    pub fn tab_bar_show_scrollbar(mut self, show: bool) -> Self {
        self.tab_bar_show_scrollbar = show;
        self
    }

    fn wrap_action(
        holder: &Rc<RefCell<DockWidgetState<K, Theme>>>,
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
        holder: &Rc<RefCell<DockWidgetState<K, Theme>>>,
        layout: &DockLayout<K>,
        node: NodeId,
    ) -> Option<Element<'static, Message, Theme, Renderer>> {
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
                let h = Rc::clone(holder);
                let on_event = Rc::clone(&self.on_event);
                let on_split =
                    Rc::new(move |action: DockAction| Self::wrap_action(&h, &on_event, action));
                Some(
                    SplitContainer::new(
                        node,
                        pg.axis,
                        pg.proportions.clone(),
                        children,
                        on_split,
                        Rc::clone(&self.class),
                        Self::theme_cell(holder),
                        self.min_pane_width,
                        self.min_pane_height,
                    )
                    .into(),
                )
            }
            NodeKind::Pane(p) => self.build_pane(holder, layout, node, p),
            NodeKind::Panel(_) => None,
            NodeKind::Root(_) => {
                let c = layout.root_child()?;
                self.build_node(holder, layout, c)
            }
        }
    }

    fn build_pane(
        &self,
        holder: &Rc<RefCell<DockWidgetState<K, Theme>>>,
        layout: &DockLayout<K>,
        pane_id: NodeId,
        pane: &Pane,
    ) -> Option<Element<'static, Message, Theme, Renderer>> {
        let active = pane.active.or_else(|| pane.tabs.first().copied())?;
        let entry = layout.get(active)?;
        let content_key: K = match &entry.kind {
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

        let pane_content = (self.content)(content_key);
        let pane_class = pane_content
            .style
            .map_or_else(|| Rc::clone(&self.class), Rc::new);
        let content = pane_content.element;

        let h = Rc::clone(holder);
        let on_event = Rc::clone(&self.on_event);
        let on_tab = Rc::new(move |action: DockAction| Self::wrap_action(&h, &on_event, action));
        Some(
            TabDock::new(
                Rc::clone(holder),
                pane_id,
                tabs,
                active,
                content,
                on_tab,
                pane_class,
                Self::theme_cell(holder),
                self.drag_threshold,
                self.drop_edge_fraction,
                self.tab_bar_scrollbar_hide_delay,
                self.tab_bar_show_scrollbar,
            )
            .into(),
        )
    }

    fn build_root_child(
        &self,
        holder: &Rc<RefCell<DockWidgetState<K, Theme>>>,
    ) -> Option<Element<'static, Message, Theme, Renderer>> {
        let state = holder.borrow();
        let root = state.layout.root_child()?;
        self.build_node(holder, &state.layout, root)
    }

    /// Rebuild cached root element and reconcile `tree.children`.
    fn sync_root(&self, tree: &mut Tree) {
        let dock_state = tree
            .state
            .downcast_ref::<DockTreeHolder<K, Message, Theme, Renderer>>()
            .dock_state
            .clone();
        let new_root = self.build_root_child(&dock_state);
        {
            let holder = tree
                .state
                .downcast_mut::<DockTreeHolder<K, Message, Theme, Renderer>>();
            holder.root = new_root;
        }
        if let Some(child) = tree
            .state
            .downcast_ref::<DockTreeHolder<K, Message, Theme, Renderer>>()
            .root
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
        f: impl FnOnce(&Element<'static, Message, Theme, Renderer>) -> R,
    ) -> Option<R> {
        let holder = tree
            .state
            .downcast_ref::<DockTreeHolder<K, Message, Theme, Renderer>>();
        holder.root.as_ref().map(f)
    }
}

type ContentFn<K, Message, Theme, Renderer> =
    Rc<dyn Fn(K) -> PaneContent<'static, Message, Theme, Renderer>>;

pub struct DockBuilder<K, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Theme: Catalog,
    Renderer: advanced::Renderer,
{
    content: Option<ContentFn<K, Message, Theme, Renderer>>,
    on_event: Option<Rc<dyn Fn(DockEvent) -> Message>>,
    shared_state: Option<Rc<RefCell<DockWidgetState<K, Theme>>>>,
    class: Option<Rc<<Theme as Catalog>::Class<'static>>>,
    min_pane_width: f32,
    min_pane_height: f32,
    drag_threshold: f32,
    drop_edge_fraction: f32,
    tab_bar_scrollbar_hide_delay: Duration,
    tab_bar_show_scrollbar: bool,
}

impl<K, Message, Theme, Renderer> Default for DockBuilder<K, Message, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: advanced::Renderer,
{
    fn default() -> Self {
        Self {
            content: None,
            on_event: None,
            shared_state: None,
            class: None,
            min_pane_width: 80.0,
            min_pane_height: 80.0,
            drag_threshold: 6.0,
            drop_edge_fraction: 0.2,
            tab_bar_scrollbar_hide_delay: Duration::from_secs(1),
            tab_bar_show_scrollbar: false,
        }
    }
}

impl<K, Message, Theme, Renderer> DockBuilder<K, Message, Theme, Renderer>
where
    K: Copy + 'static,
    Message: Clone + 'static,
    Theme: Catalog
        + button::Catalog
        + container::Catalog
        + iced_text::Catalog
        + menu::Catalog
        + Clone
        + PartialEq
        + 'static,
    Renderer: advanced::Renderer + advanced::text::Renderer + 'static,
    <Theme as button::Catalog>::Class<'static>: From<button::StyleFn<'static, Theme>>,
    <Theme as container::Catalog>::Class<'static>: From<container::StyleFn<'static, Theme>>,
    for<'b> <Theme as iced_text::Catalog>::Class<'b>: From<iced_text::StyleFn<'b, Theme>>,
{
    #[must_use]
    pub fn content(
        mut self,
        f: impl Fn(K) -> Element<'static, Message, Theme, Renderer> + 'static,
    ) -> Self {
        self.content = Some(Rc::new(move |key| PaneContent::from(f(key))));
        self
    }

    /// Like [`content`](Self::content), but the closure returns [`PaneContent`]
    /// for per-pane style overrides.
    #[must_use]
    pub fn content_styled(
        mut self,
        f: impl Fn(K) -> PaneContent<'static, Message, Theme, Renderer> + 'static,
    ) -> Self {
        self.content = Some(Rc::new(f));
        self
    }

    /// Map observation [`DockEvent`] values to the application message type.
    ///
    /// The widget applies layout mutations before this callback; do not call
    /// [`DockSession::dispatch`](crate::DockSession::dispatch) for widget-originated events.
    #[must_use]
    pub fn on_event(mut self, f: impl Fn(DockEvent) -> Message + 'static) -> Self {
        self.on_event = Some(Rc::new(f));
        self
    }

    #[must_use]
    pub fn state(mut self, state: Rc<RefCell<DockWidgetState<K, Theme>>>) -> Self {
        self.shared_state = Some(state);
        self
    }

    #[must_use]
    pub fn style(mut self, style: impl Fn(&Theme) -> DockStyle + 'static) -> Self
    where
        <Theme as Catalog>::Class<'static>: From<StyleFn<'static, Theme>>,
    {
        self.class = Some(Rc::new((Box::new(style) as StyleFn<'static, Theme>).into()));
        self
    }

    /// Sets the style class of the [`Dock`].
    #[must_use]
    pub fn class(mut self, class: <Theme as Catalog>::Class<'static>) -> Self {
        self.class = Some(Rc::new(class));
        self
    }

    /// Minimum width of each pane in horizontal split groups.
    ///
    /// Split drags stop when an adjacent pair would shrink a pane below this width.
    /// Default is `80.0`.
    #[must_use]
    pub fn min_pane_width(mut self, min_pane_width: f32) -> Self {
        self.min_pane_width = min_pane_width.max(1.0);
        self
    }

    /// Minimum height of each pane in vertical split groups.
    ///
    /// Split drags stop when an adjacent pair would shrink a pane below this height.
    /// Default is `80.0`.
    #[must_use]
    pub fn min_pane_height(mut self, min_pane_height: f32) -> Self {
        self.min_pane_height = min_pane_height.max(1.0);
        self
    }

    /// Minimum pointer movement before a tab label press becomes a dock drag.
    ///
    /// Default is `6.0`.
    #[must_use]
    pub fn drag_threshold(mut self, threshold: f32) -> Self {
        self.drag_threshold = threshold.max(0.0);
        self
    }

    /// Fraction of pane edge used for edge drop bands (0.0–0.5).
    ///
    /// Default is `0.2`.
    #[must_use]
    pub fn drop_edge_fraction(mut self, fraction: f32) -> Self {
        self.drop_edge_fraction = fraction.clamp(0.0, 0.5);
        self
    }

    /// Delay before the tab-bar scrollbar hides after the pointer leaves the tab bar.
    ///
    /// Default is one second.
    #[must_use]
    pub fn tab_bar_scrollbar_hide_delay(mut self, delay: Duration) -> Self {
        self.tab_bar_scrollbar_hide_delay = delay;
        self
    }

    /// Whether overflowing tab bars show a horizontal scrollbar thumb.
    ///
    /// When `false`, tabs can still be scrolled with the mouse wheel (and Shift+wheel).
    /// Default is `true`.
    #[must_use]
    pub fn tab_bar_show_scrollbar(mut self, show: bool) -> Self {
        self.tab_bar_show_scrollbar = show;
        self
    }

    /// # Panics
    ///
    /// Panics when [`on_event`](Self::on_event) was not set.
    #[must_use]
    pub fn build(self) -> Dock<K, Message, Theme, Renderer> {
        let content: ContentFn<K, Message, Theme, Renderer> = self
            .content
            .unwrap_or_else(|| Rc::new(|_| PaneContent::new(widget::text("No content"))));
        let on_event = self
            .on_event
            .unwrap_or_else(|| Rc::new(|_| panic!("dock().on_event(...) required")));
        Dock {
            content,
            on_event,
            external_state: self.shared_state,
            class: self
                .class
                .unwrap_or_else(|| Rc::new(<Theme as Catalog>::default())),
            min_pane_width: self.min_pane_width,
            min_pane_height: self.min_pane_height,
            drag_threshold: self.drag_threshold,
            drop_edge_fraction: self.drop_edge_fraction,
            tab_bar_scrollbar_hide_delay: self.tab_bar_scrollbar_hide_delay,
            tab_bar_show_scrollbar: self.tab_bar_show_scrollbar,
        }
    }
}

#[must_use]
pub fn dock<K, Message, Theme, Renderer>() -> DockBuilder<K, Message, Theme, Renderer>
where
    K: Copy + 'static,
    Message: Clone + 'static,
    Theme: Catalog
        + button::Catalog
        + container::Catalog
        + iced_text::Catalog
        + menu::Catalog
        + Clone
        + PartialEq
        + 'static,
    Renderer: advanced::Renderer + advanced::text::Renderer + 'static,
    <Theme as button::Catalog>::Class<'static>: From<button::StyleFn<'static, Theme>>,
    <Theme as container::Catalog>::Class<'static>: From<container::StyleFn<'static, Theme>>,
    for<'b> <Theme as iced_text::Catalog>::Class<'b>: From<iced_text::StyleFn<'b, Theme>>,
{
    DockBuilder::default()
}

impl<K, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Dock<K, Message, Theme, Renderer>
where
    K: Copy + 'static,
    Message: Clone + 'static,
    Theme: Catalog
        + button::Catalog
        + container::Catalog
        + iced_text::Catalog
        + menu::Catalog
        + Clone
        + PartialEq
        + 'static,
    Renderer: advanced::Renderer + advanced::text::Renderer + 'static,
    <Theme as button::Catalog>::Class<'static>: From<button::StyleFn<'static, Theme>>,
    <Theme as container::Catalog>::Class<'static>: From<container::StyleFn<'static, Theme>>,
    for<'b> <Theme as iced_text::Catalog>::Class<'b>: From<iced_text::StyleFn<'b, Theme>>,
{
    fn tag(&self) -> Tag {
        Tag::of::<DockTreeHolder<K, Message, Theme, Renderer>>()
    }

    fn state(&self) -> State {
        let dock_state = self
            .external_state
            .clone()
            .unwrap_or_else(|| Rc::new(RefCell::new(DockWidgetState::default())));
        State::new(DockTreeHolder::<K, Message, Theme, Renderer> {
            dock_state,
            root: None,
        })
    }

    fn diff(&self, tree: &mut Tree) {
        let dirty = tree
            .state
            .downcast_ref::<DockTreeHolder<K, Message, Theme, Renderer>>()
            .dock_state
            .borrow()
            .layout_dirty;
        if dirty {
            tree.state
                .downcast_ref::<DockTreeHolder<K, Message, Theme, Renderer>>()
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
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let dock_state = tree
            .state
            .downcast_ref::<DockTreeHolder<K, Message, Theme, Renderer>>()
            .dock_state
            .clone();
        dock_state.borrow_mut().drop_targets.clear();
        dock_state.borrow_mut().tab_bar_targets.clear();

        if tree.children.is_empty() {
            self.sync_root(tree);
        }
        let size = limits.max();
        if let Some(child_tree) = tree.children.first_mut() {
            if let Some(child) = tree
                .state
                .downcast_mut::<DockTreeHolder<K, Message, Theme, Renderer>>()
                .root
                .as_mut()
            {
                let child_node = child.as_widget_mut().layout(child_tree, renderer, limits);
                return layout::Node::with_children(size, vec![child_node]);
            }
        }
        layout::Node::new(size)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
    ) {
        let dock_state = tree
            .state
            .downcast_ref::<DockTreeHolder<K, Message, Theme, Renderer>>()
            .dock_state
            .clone();
        *dock_state.borrow_mut().resolved_theme.borrow_mut() = Some(theme.clone());

        let dock_style = Catalog::style(theme, &self.class);
        renderer.fill_quad(
            renderer::Quad {
                bounds: layout.bounds(),
                ..renderer::Quad::default()
            },
            dock_style.background.color,
        );

        tree.state
            .downcast_ref::<DockTreeHolder<K, Message, Theme, Renderer>>()
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
        Self::with_cached_root(tree, |child| {
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
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let dock_state = tree
            .state
            .downcast_ref::<DockTreeHolder<K, Message, Theme, Renderer>>()
            .dock_state
            .clone();

        let Some(child_layout) = layout.children().next() else {
            return;
        };
        let Some(child_tree) = tree.children.first_mut() else {
            return;
        };
        if let Some(child) = tree
            .state
            .downcast_mut::<DockTreeHolder<K, Message, Theme, Renderer>>()
            .root
            .as_mut()
        {
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
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let holder = tree
            .state
            .downcast_ref::<DockTreeHolder<K, Message, Theme, Renderer>>();
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
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        let Some(child_layout) = layout.children().next() else {
            return;
        };
        let Some(child_tree) = tree.children.first_mut() else {
            return;
        };
        if let Some(child) = tree
            .state
            .downcast_mut::<DockTreeHolder<K, Message, Theme, Renderer>>()
            .root
            .as_mut()
        {
            child
                .as_widget_mut()
                .operate(child_tree, child_layout, renderer, operation);
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let child_layout = layout.children().next()?;
        let child_tree = tree.children.first_mut()?;
        let child = tree
            .state
            .downcast_mut::<DockTreeHolder<K, Message, Theme, Renderer>>()
            .root
            .as_mut()?;
        child
            .as_widget_mut()
            .overlay(child_tree, child_layout, renderer, viewport, translation)
    }
}

impl<K, Message, Theme, Renderer> From<Dock<K, Message, Theme, Renderer>>
    for Element<'static, Message, Theme, Renderer>
where
    K: Copy + 'static,
    Message: Clone + 'static,
    Theme: Catalog
        + button::Catalog
        + container::Catalog
        + iced_text::Catalog
        + menu::Catalog
        + Clone
        + PartialEq
        + 'static,
    Renderer: advanced::Renderer + advanced::text::Renderer + 'static,
    <Theme as button::Catalog>::Class<'static>: From<button::StyleFn<'static, Theme>>,
    <Theme as container::Catalog>::Class<'static>: From<container::StyleFn<'static, Theme>>,
    for<'b> <Theme as iced_text::Catalog>::Class<'b>: From<iced_text::StyleFn<'b, Theme>>,
{
    fn from(widget: Dock<K, Message, Theme, Renderer>) -> Self {
        Element::new(widget)
    }
}
