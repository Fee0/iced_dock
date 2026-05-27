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

use crate::model::{Layout as ModelLayout, NodeId, NodeKind, Pane};
use crate::style::{Catalog, DockStyle, PaneContent, StyleFn};
use crate::widget::action::DockAction;
use crate::widget::event::{action_to_event, DockEvent};
use crate::widget::split::SplitContainer;
use crate::widget::state::{dispatch_action, DockWidgetState};
use crate::widget::tab_dock::{TabDock, TabInfo};

/// Vertical attachment edge for the optional tab-bar scrollbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TabBarScrollbarAttachment {
    /// Render the scrollbar flush with the top edge of the tab bar.
    #[default]
    Top,
    /// Render the scrollbar flush with the bottom edge of the tab bar.
    Bottom,
}

/// Persistent tree state shared across frames. Holds the dock layout state
/// and a cached theme reference for the layout pass (which doesn't receive `&Theme`).
struct DockTreeHolder<K, Theme>
where
    Theme: Catalog,
{
    dock_state: Rc<RefCell<DockWidgetState<K>>>,
    resolved_theme: Rc<RefCell<Option<Theme>>>,
}

/// The top-level docking widget.
///
/// `Dock` renders a full split/tab layout from a [`DockWidgetState`] and
/// rebuilds its internal element tree each layout pass (following the same
/// pattern as iced's `responsive()` widget).
///
/// Use the [`dock()`] free function to obtain a [`DockBuilder`] for
/// ergonomic construction.
///
/// # Type parameters
///
/// * `'a` — View lifetime (matches the application's `view(&self)` borrow).
/// * `K` — Content key type stored in each panel (e.g. an enum of panel kinds).
/// * `Message` — The application message type.
/// * `Theme` — The iced theme (must implement [`Catalog`]).
/// * `Renderer` — The iced renderer.
pub struct Dock<'a, K, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Theme: Catalog,
    Renderer: advanced::Renderer + advanced::text::Renderer,
{
    content: Box<dyn Fn(K) -> PaneContent<'a, Message, Theme, Renderer> + 'a>,
    on_event: Rc<dyn Fn(DockEvent<K>) -> Message>,
    external_state: Option<Rc<RefCell<DockWidgetState<K>>>>,
    class: Rc<<Theme as Catalog>::Class<'static>>,
    root: Element<'a, Message, Theme, Renderer>,
    tab_bar_height: f32,
    tab_bar_spacing: f32,
    tab_bar_padding: [f32; 2],
    tab_text_size: f32,
    tab_font: Option<Renderer::Font>,
    tab_padding: [f32; 2],
    tab_accent_height: f32,
    close_button_text_size: f32,
    close_button_size: f32,
    close_button_margin_right: f32,
    close_button_padding: [f32; 2],
    splitter_size: f32,
    splitter_gap: f32,
    pane_padding: f32,
    scrollbar_height: f32,
    scrollbar_thumb_min_width: f32,
    insert_marker_width: f32,
    separator_height: f32,
    min_pane_width: f32,
    min_pane_height: f32,
    drag_threshold: f32,
    drop_edge_fraction: f32,
    tab_bar_scrollbar_fade_duration: Duration,
    tab_bar_scrollbar_animated: bool,
    tab_bar_show_scrollbar: bool,
    tab_bar_scrollbar_attachment: TabBarScrollbarAttachment,
}

impl<'a, K, Message, Theme, Renderer> Dock<'a, K, Message, Theme, Renderer>
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
    /// Override the dock chrome style with a closure.
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

    /// Attach shared widget state so the dock reads layout from an external
    /// [`DockWidgetState`] (typically obtained from [`DockSession::state`](crate::DockSession::state)).
    #[must_use]
    pub fn with_state(mut self, state: Rc<RefCell<DockWidgetState<K>>>) -> Self {
        self.external_state = Some(state);
        self
    }

    /// Fade duration for the tab-bar scrollbar when its visibility changes.
    ///
    /// Default is 0.5 seconds.
    #[must_use]
    pub fn tab_bar_scrollbar_fade_duration(mut self, duration: Duration) -> Self {
        self.tab_bar_scrollbar_fade_duration = duration;
        self
    }

    /// Whether the tab-bar scrollbar fades out when it hides.
    ///
    /// When `false`, the scrollbar snaps visible and hidden instantly.
    /// Default is `true`.
    #[must_use]
    pub fn tab_bar_scrollbar_animated(mut self, animated: bool) -> Self {
        self.tab_bar_scrollbar_animated = animated;
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

    /// Vertical edge used to attach the optional tab-bar scrollbar.
    ///
    /// Default is [`TabBarScrollbarAttachment::Top`].
    #[must_use]
    pub fn tab_bar_scrollbar_attachment(mut self, attachment: TabBarScrollbarAttachment) -> Self {
        self.tab_bar_scrollbar_attachment = attachment;
        self
    }

    fn wrap_action(
        holder: &Rc<RefCell<DockWidgetState<K>>>,
        on_event: &Rc<dyn Fn(DockEvent<K>) -> Message>,
        action: DockAction,
    ) -> Message {
        let mut state = holder.borrow_mut();
        let event = action_to_event(&state.layout, &action).unwrap_or(DockEvent::LayoutChanged);
        dispatch_action(&mut state, action);
        (on_event)(event)
    }

    fn build_node(
        &self,
        holder: &Rc<RefCell<DockWidgetState<K>>>,
        theme_cell: &Rc<RefCell<Option<Theme>>>,
        layout: &ModelLayout<K>,
        node: NodeId,
    ) -> Option<Element<'a, Message, Theme, Renderer>> {
        match layout.kind(node)? {
            NodeKind::Proportional(pg) => {
                let children: Vec<_> = pg
                    .children
                    .iter()
                    .filter_map(|&c| self.build_node(holder, theme_cell, layout, c))
                    .collect();
                if children.is_empty() {
                    return None;
                }
                let h = Rc::clone(holder);
                let on_ev = Rc::clone(&self.on_event);
                let on_split =
                    Rc::new(move |action: DockAction| Self::wrap_action(&h, &on_ev, action));
                Some(
                    SplitContainer::new(
                        node,
                        pg.axis,
                        pg.proportions.clone(),
                        children,
                        on_split,
                        Rc::clone(&self.class),
                        self.splitter_size,
                        self.splitter_gap,
                        self.min_pane_width,
                        self.min_pane_height,
                    )
                    .into(),
                )
            }
            NodeKind::Pane(p) => self.build_pane(holder, theme_cell, layout, node, p),
            NodeKind::Panel(_) => None,
            NodeKind::Root(_) => {
                let c = layout.root_child()?;
                self.build_node(holder, theme_cell, layout, c)
            }
        }
    }

    fn build_pane(
        &self,
        holder: &Rc<RefCell<DockWidgetState<K>>>,
        theme_cell: &Rc<RefCell<Option<Theme>>>,
        layout: &ModelLayout<K>,
        pane_id: NodeId,
        pane: &Pane,
    ) -> Option<Element<'a, Message, Theme, Renderer>> {
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
        let on_ev = Rc::clone(&self.on_event);
        let on_tab = Rc::new(move |action: DockAction| Self::wrap_action(&h, &on_ev, action));
        Some(
            TabDock::new(
                Rc::clone(holder),
                pane_id,
                tabs,
                active,
                content,
                on_tab,
                pane_class,
                Rc::clone(theme_cell),
                self.tab_bar_height,
                self.tab_bar_spacing,
                self.tab_bar_padding,
                self.tab_text_size,
                self.tab_font,
                self.tab_padding,
                self.tab_accent_height,
                self.close_button_text_size,
                self.close_button_size,
                self.close_button_margin_right,
                self.close_button_padding,
                self.pane_padding,
                self.scrollbar_height,
                self.scrollbar_thumb_min_width,
                self.insert_marker_width,
                self.separator_height,
                self.drag_threshold,
                self.drop_edge_fraction,
                self.tab_bar_scrollbar_fade_duration,
                self.tab_bar_scrollbar_animated,
                self.tab_bar_show_scrollbar,
                self.tab_bar_scrollbar_attachment,
            )
            .into(),
        )
    }

    fn build_root_element(
        &self,
        holder: &Rc<RefCell<DockWidgetState<K>>>,
        theme_cell: &Rc<RefCell<Option<Theme>>>,
    ) -> Element<'a, Message, Theme, Renderer> {
        let state = holder.borrow();
        let root = state
            .layout
            .root_child()
            .and_then(|r| self.build_node(holder, theme_cell, &state.layout, r));
        root.unwrap_or_else(|| Element::new(widget::space::Space::new()))
    }

    fn rebuild_root(&mut self, tree: &mut Tree) {
        let holder = tree.state.downcast_ref::<DockTreeHolder<K, Theme>>();
        let dock_state = Rc::clone(&holder.dock_state);
        let theme_cell = Rc::clone(&holder.resolved_theme);
        self.root = self.build_root_element(&dock_state, &theme_cell);
        tree.diff_children(std::slice::from_ref(&self.root));
    }
}

type ContentFn<'a, K, Message, Theme, Renderer> =
    Box<dyn Fn(K) -> PaneContent<'a, Message, Theme, Renderer> + 'a>;

/// Builder for constructing a [`Dock`] widget with ergonomic chained setters.
///
/// Obtained via [`dock()`]. At minimum, call [`content`](Self::content),
/// [`on_event`](Self::on_event), [`state`](Self::state), and [`build`](Self::build):
///
/// ```ignore
/// dock()
///     .state(session.state())
///     .on_event(Message::DockEvent)
///     .content(|key| view_panel(key))
///     .build()
/// ```
pub struct DockBuilder<'a, K, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Theme: Catalog,
    Renderer: advanced::Renderer + advanced::text::Renderer,
{
    content: Option<ContentFn<'a, K, Message, Theme, Renderer>>,
    on_event: Option<Rc<dyn Fn(DockEvent<K>) -> Message>>,
    shared_state: Option<Rc<RefCell<DockWidgetState<K>>>>,
    class: Option<Rc<<Theme as Catalog>::Class<'static>>>,
    tab_bar_height: f32,
    tab_bar_spacing: f32,
    tab_bar_padding: [f32; 2],
    tab_text_size: f32,
    tab_font: Option<Renderer::Font>,
    tab_padding: [f32; 2],
    tab_accent_height: f32,
    close_button_text_size: f32,
    close_button_size: f32,
    close_button_margin_right: f32,
    close_button_padding: [f32; 2],
    splitter_size: f32,
    splitter_gap: f32,
    pane_padding: f32,
    scrollbar_height: f32,
    scrollbar_thumb_min_width: f32,
    insert_marker_width: f32,
    separator_height: f32,
    min_pane_width: f32,
    min_pane_height: f32,
    drag_threshold: f32,
    drop_edge_fraction: f32,
    tab_bar_scrollbar_fade_duration: Duration,
    tab_bar_scrollbar_animated: bool,
    tab_bar_show_scrollbar: bool,
    tab_bar_scrollbar_attachment: TabBarScrollbarAttachment,
}

impl<'a, K, Message, Theme, Renderer> Default for DockBuilder<'a, K, Message, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: advanced::Renderer + advanced::text::Renderer,
{
    fn default() -> Self {
        Self {
            content: None,
            on_event: None,
            shared_state: None,
            class: None,
            tab_bar_height: 30.0,
            tab_bar_spacing: 0.0,
            tab_bar_padding: [0.0, 0.0],
            tab_text_size: 12.0,
            tab_font: None,
            tab_padding: [0.0, 10.0],
            tab_accent_height: 2.0,
            close_button_text_size: 15.0,
            close_button_size: 20.0,
            close_button_margin_right: 6.0,
            close_button_padding: [0.0, 0.0],
            splitter_size: 0.5,
            splitter_gap: 10.0,
            pane_padding: 0.0,
            scrollbar_height: 6.0,
            scrollbar_thumb_min_width: 24.0,
            insert_marker_width: 3.0,
            separator_height: 1.0,
            min_pane_width: 80.0,
            min_pane_height: 80.0,
            drag_threshold: 6.0,
            drop_edge_fraction: 0.2,
            tab_bar_scrollbar_fade_duration: Duration::from_millis(500),
            tab_bar_scrollbar_animated: true,
            tab_bar_show_scrollbar: false,
            tab_bar_scrollbar_attachment: TabBarScrollbarAttachment::Top,
        }
    }
}

impl<'a, K, Message, Theme, Renderer> DockBuilder<'a, K, Message, Theme, Renderer>
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
    /// Set the content factory that maps a panel key `K` to its [`Element`].
    ///
    /// The closure may borrow from application state; it only needs to live
    /// as long as the view frame (`'a`).
    #[must_use]
    pub fn content(mut self, f: impl Fn(K) -> Element<'a, Message, Theme, Renderer> + 'a) -> Self {
        self.content = Some(Box::new(move |key| PaneContent::from(f(key))));
        self
    }

    /// Like [`content`](Self::content), but the closure returns [`PaneContent`]
    /// for per-pane style overrides.
    #[must_use]
    pub fn content_styled(
        mut self,
        f: impl Fn(K) -> PaneContent<'a, Message, Theme, Renderer> + 'a,
    ) -> Self {
        self.content = Some(Box::new(f));
        self
    }

    /// Map observation [`DockEvent`] values to the application message type.
    ///
    /// The widget applies layout mutations before this callback; do not call
    /// [`DockSession::dispatch`](crate::DockSession::dispatch) for widget-originated events.
    #[must_use]
    pub fn on_event(mut self, f: impl Fn(DockEvent<K>) -> Message + 'static) -> Self {
        self.on_event = Some(Rc::new(f));
        self
    }

    /// Attach shared widget state (typically obtained from
    /// [`DockSession::state`](crate::DockSession::state)).
    #[must_use]
    pub fn state(mut self, state: Rc<RefCell<DockWidgetState<K>>>) -> Self {
        self.shared_state = Some(state);
        self
    }

    /// Override the dock chrome style with a closure.
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

    /// Fade duration for the tab-bar scrollbar when its visibility changes.
    ///
    /// Default is 0.5 seconds.
    #[must_use]
    pub fn tab_bar_scrollbar_fade_duration(mut self, duration: Duration) -> Self {
        self.tab_bar_scrollbar_fade_duration = duration;
        self
    }

    /// Whether the tab-bar scrollbar fades out when it hides.
    ///
    /// When `false`, the scrollbar snaps visible and hidden instantly.
    /// Default is `true`.
    #[must_use]
    pub fn tab_bar_scrollbar_animated(mut self, animated: bool) -> Self {
        self.tab_bar_scrollbar_animated = animated;
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

    /// Vertical edge used to attach the optional tab-bar scrollbar.
    ///
    /// Default is [`TabBarScrollbarAttachment::Top`].
    #[must_use]
    pub fn tab_bar_scrollbar_attachment(mut self, attachment: TabBarScrollbarAttachment) -> Self {
        self.tab_bar_scrollbar_attachment = attachment;
        self
    }

    /// Height of the tab bar strip above each pane. Default `30.0`.
    #[must_use]
    pub fn tab_bar_height(mut self, h: f32) -> Self {
        self.tab_bar_height = h.max(1.0);
        self
    }

    /// Horizontal spacing between adjacent tabs. Default `0.0`.
    #[must_use]
    pub fn tab_bar_spacing(mut self, s: f32) -> Self {
        self.tab_bar_spacing = s.max(0.0);
        self
    }

    /// Outer padding of the tab bar: `[vertical, horizontal]`. Default `[0, 0]`.
    #[must_use]
    pub fn tab_bar_padding(mut self, p: [f32; 2]) -> Self {
        self.tab_bar_padding = p;
        self
    }

    /// Font size for tab labels. Default `12.0`.
    #[must_use]
    pub fn tab_text_size(mut self, s: f32) -> Self {
        self.tab_text_size = s.max(1.0);
        self
    }

    /// Font for tab labels. When unset, tab text uses the renderer's
    /// [`default_font`](iced::Settings::default_font) (same as plain `text()`).
    #[must_use]
    pub fn tab_font(mut self, font: impl Into<Renderer::Font>) -> Self {
        self.tab_font = Some(font.into());
        self
    }

    /// Inner padding of each tab label: `[vertical, horizontal]`. Default `[0, 10]`.
    #[must_use]
    pub fn tab_padding(mut self, p: [f32; 2]) -> Self {
        self.tab_padding = p;
        self
    }

    /// Height of the accent bar drawn under the active tab. Default `2.0`.
    #[must_use]
    pub fn tab_accent_height(mut self, h: f32) -> Self {
        self.tab_accent_height = h.max(0.0);
        self
    }

    /// Square size of the close button. Default `20.0`.
    #[must_use]
    pub fn close_button_size(mut self, s: f32) -> Self {
        self.close_button_size = s.max(1.0);
        self
    }

    /// Space between the close button and the tab edge. Default `6.0`.
    #[must_use]
    pub fn close_button_margin(mut self, m: f32) -> Self {
        self.close_button_margin_right = m.max(0.0);
        self
    }

    /// Visual thickness of the splitter divider line. Default `0.5`.
    #[must_use]
    pub fn splitter_size(mut self, s: f32) -> Self {
        self.splitter_size = s.max(0.0);
        self
    }

    /// Extra gap between panes. Default `10.0`.
    #[must_use]
    pub fn splitter_gap(mut self, g: f32) -> Self {
        self.splitter_gap = g.max(0.0);
        self
    }

    /// Content padding inside each pane. Default `0.0`.
    #[must_use]
    pub fn pane_padding(mut self, p: f32) -> Self {
        self.pane_padding = p.max(0.0);
        self
    }

    /// Height of the scrollbar thumb in overflowing tab bars. Default `6.0`.
    #[must_use]
    pub fn scrollbar_height(mut self, h: f32) -> Self {
        self.scrollbar_height = h.max(1.0);
        self
    }

    /// Width of the vertical insertion marker during tab drag. Default `3.0`.
    #[must_use]
    pub fn insert_marker_width(mut self, w: f32) -> Self {
        self.insert_marker_width = w.max(1.0);
        self
    }

    /// Height of the separator line at the bottom of the tab bar. Default `1.0`.
    #[must_use]
    pub fn separator_height(mut self, h: f32) -> Self {
        self.separator_height = h.max(0.0);
        self
    }

    /// # Panics
    ///
    /// Panics when [`on_event`](Self::on_event) was not set.
    #[must_use]
    pub fn build(self) -> Dock<'a, K, Message, Theme, Renderer> {
        let content: ContentFn<'a, K, Message, Theme, Renderer> = self
            .content
            .unwrap_or_else(|| Box::new(|_| PaneContent::new(widget::text("No content"))));
        let on_event: Rc<dyn Fn(DockEvent<K>) -> Message> = self
            .on_event
            .unwrap_or_else(|| Rc::new(|_| panic!("dock().on_event(...) required")));
        Dock {
            content,
            on_event,
            external_state: self.shared_state,
            class: self
                .class
                .unwrap_or_else(|| Rc::new(<Theme as Catalog>::default())),
            root: Element::new(widget::space::Space::new()),
            tab_bar_height: self.tab_bar_height,
            tab_bar_spacing: self.tab_bar_spacing,
            tab_bar_padding: self.tab_bar_padding,
            tab_text_size: self.tab_text_size,
            tab_font: self.tab_font,
            tab_padding: self.tab_padding,
            tab_accent_height: self.tab_accent_height,
            close_button_text_size: self.close_button_text_size,
            close_button_size: self.close_button_size,
            close_button_margin_right: self.close_button_margin_right,
            close_button_padding: self.close_button_padding,
            splitter_size: self.splitter_size,
            splitter_gap: self.splitter_gap,
            pane_padding: self.pane_padding,
            scrollbar_height: self.scrollbar_height,
            scrollbar_thumb_min_width: self.scrollbar_thumb_min_width,
            insert_marker_width: self.insert_marker_width,
            separator_height: self.separator_height,
            min_pane_width: self.min_pane_width,
            min_pane_height: self.min_pane_height,
            drag_threshold: self.drag_threshold,
            drop_edge_fraction: self.drop_edge_fraction,
            tab_bar_scrollbar_fade_duration: self.tab_bar_scrollbar_fade_duration,
            tab_bar_scrollbar_animated: self.tab_bar_scrollbar_animated,
            tab_bar_show_scrollbar: self.tab_bar_show_scrollbar,
            tab_bar_scrollbar_attachment: self.tab_bar_scrollbar_attachment,
        }
    }
}

/// Create a [`DockBuilder`] for constructing a [`Dock`] widget.
///
/// This is the primary entry point for building a dock layout in your view
/// function. See [`DockBuilder`] for the full set of configuration options.
#[must_use]
pub fn dock<'a, K, Message, Theme, Renderer>() -> DockBuilder<'a, K, Message, Theme, Renderer>
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
    for Dock<'_, K, Message, Theme, Renderer>
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
        Tag::of::<DockTreeHolder<K, Theme>>()
    }

    fn state(&self) -> State {
        let dock_state = self
            .external_state
            .clone()
            .unwrap_or_else(|| Rc::new(RefCell::new(DockWidgetState::default())));
        State::new(DockTreeHolder::<K, Theme> {
            dock_state,
            resolved_theme: Rc::new(RefCell::new(None)),
        })
    }

    fn diff(&self, _tree: &mut Tree) {
        // Deferred to layout(), following the responsive() pattern.
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
        let holder = tree.state.downcast_ref::<DockTreeHolder<K, Theme>>();
        let dock_state = Rc::clone(&holder.dock_state);
        dock_state.borrow_mut().commit_layout();
        dock_state.borrow_mut().drop_targets.clear();
        dock_state.borrow_mut().tab_bar_targets.clear();

        self.rebuild_root(tree);

        let size = limits.max();
        if let Some(child_tree) = tree.children.first_mut() {
            let child_node = self
                .root
                .as_widget_mut()
                .layout(child_tree, renderer, limits);
            layout::Node::with_children(size, vec![child_node])
        } else {
            layout::Node::new(size)
        }
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
        let holder = tree.state.downcast_ref::<DockTreeHolder<K, Theme>>();
        *holder.resolved_theme.borrow_mut() = Some(theme.clone());
        holder.dock_state.borrow_mut().pane_bounds.clear();

        let dock_style = Catalog::style(theme, &self.class);
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
        self.root.as_widget().draw(
            child_tree,
            renderer,
            theme,
            style,
            child_layout,
            cursor,
            viewport,
        );
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
        let Some(child_layout) = layout.children().next() else {
            return;
        };
        let Some(child_tree) = tree.children.first_mut() else {
            return;
        };
        self.root.as_widget_mut().update(
            child_tree,
            event,
            child_layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        let dock_state = Rc::clone(
            &tree
                .state
                .downcast_ref::<DockTreeHolder<K, Theme>>()
                .dock_state,
        );

        if dock_state.borrow().layout_dirty {
            dock_state.borrow_mut().commit_layout();
            dock_state.borrow_mut().pane_bounds.clear();
            self.rebuild_root(tree);
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
        let holder = tree.state.downcast_ref::<DockTreeHolder<K, Theme>>();
        if holder.dock_state.borrow().drag.is_some() {
            return mouse::Interaction::Grab;
        }

        let Some(child_layout) = layout.children().next() else {
            return mouse::Interaction::default();
        };
        let Some(child_tree) = tree.children.first() else {
            return mouse::Interaction::default();
        };
        self.root.as_widget().mouse_interaction(
            child_tree,
            child_layout,
            cursor,
            viewport,
            renderer,
        )
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
        self.root
            .as_widget_mut()
            .operate(child_tree, child_layout, renderer, operation);
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
        self.root
            .as_widget_mut()
            .overlay(child_tree, child_layout, renderer, viewport, translation)
    }
}

impl<'a, K, Message, Theme, Renderer> From<Dock<'a, K, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
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
    fn from(widget: Dock<'a, K, Message, Theme, Renderer>) -> Self {
        Element::new(widget)
    }
}
