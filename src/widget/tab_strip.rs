use std::cell::RefCell;
use std::rc::Rc;

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::tree::{State, Tag, Tree};
use iced::advanced::widget::{Operation, Widget};
use iced::advanced::{Clipboard, Shell};
use iced::keyboard;
use iced::mouse::{self, Cursor};
use iced::widget::{button, container, mouse_area, row, text, Space};
use iced::window;
use iced::{
    Border, Color, Element, Event, Length, Padding, Rectangle, Size, Vector,
};

use crate::model::NodeId;
use crate::style::{close_button_style, Catalog, CloseButtonStyle, DockStyle};
use crate::widget::compose;
use crate::widget::action::{DockAction, TabAction};
use crate::widget::tab_dock::TabInfo;

#[derive(Debug, Clone, Copy)]
struct ScrollbarDrag {
    grab_x: f32,
}

struct TabStripState<Theme> {
    scroll_offset: f32,
    content_width: f32,
    viewport_width: f32,
    tab_bar_hovered: bool,
    scrollbar_visible: bool,
    hide_at: Option<iced::time::Instant>,
    scrollbar_drag: Option<ScrollbarDrag>,
    scrollbar_thumb_hovered: bool,
    keyboard_modifiers: keyboard::Modifiers,
    drag_pending: bool,
    drag_start: Option<iced::Point>,
    dragging: bool,
    pressed_tab: Option<NodeId>,
    hovered_tab: Option<NodeId>,
    insert_marker_index: Option<usize>,
    /// When true, tab label and close-button hover are disabled (active global tab drag).
    suppress_hover: bool,
    /// Theme used for the last [`build_tabs_row`] rebuild.
    built_theme: Option<Theme>,
}

impl<Theme> TabStripState<Theme> {
    fn new(theme: Option<Theme>) -> Self {
        Self {
            scroll_offset: 0.0,
            content_width: 0.0,
            viewport_width: 0.0,
            tab_bar_hovered: false,
            scrollbar_visible: false,
            hide_at: None,
            scrollbar_drag: None,
            scrollbar_thumb_hovered: false,
            keyboard_modifiers: keyboard::Modifiers::default(),
            drag_pending: false,
            drag_start: None,
            dragging: false,
            pressed_tab: None,
            hovered_tab: None,
            insert_marker_index: None,
            suppress_hover: false,
            built_theme: theme,
        }
    }
}

pub struct TabStrip<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Theme: Catalog,
    Renderer: iced::advanced::Renderer,
{
    pane_id: NodeId,
    tabs: Vec<TabInfo>,
    active_tab: NodeId,
    tabs_row: Element<'a, Message, Theme, Renderer>,
    on_event: Rc<dyn Fn(DockAction) -> Message>,
    class: Rc<<Theme as Catalog>::Class<'static>>,
    drag_threshold: f32,
    drop_edge_fraction: f32,
    hide_delay: iced::time::Duration,
    show_scrollbar: bool,
    theme: Rc<RefCell<Option<Theme>>>,
}

impl<'a, Message, Theme, Renderer> TabStrip<'a, Message, Theme, Renderer>
where
    Message: Clone + 'static,
    Theme: Catalog
        + iced::widget::button::Catalog
        + iced::widget::container::Catalog
        + iced::widget::text::Catalog
        + Clone
        + PartialEq
        + 'static,
    Renderer: iced::advanced::Renderer + iced::advanced::text::Renderer + 'static,
    <Theme as iced::widget::button::Catalog>::Class<'static>:
        From<iced::widget::button::StyleFn<'static, Theme>>,
    <Theme as iced::widget::container::Catalog>::Class<'static>:
        From<iced::widget::container::StyleFn<'static, Theme>>,
    for<'b> <Theme as iced::widget::text::Catalog>::Class<'b>:
        From<iced::widget::text::StyleFn<'b, Theme>>,
{
    pub fn new(
        pane_id: NodeId,
        tabs: Vec<TabInfo>,
        active_tab: NodeId,
        on_event: Rc<dyn Fn(DockAction) -> Message>,
        class: Rc<<Theme as Catalog>::Class<'static>>,
        theme: Rc<RefCell<Option<Theme>>>,
        drag_threshold: f32,
        drop_edge_fraction: f32,
        hide_delay: iced::time::Duration,
        show_scrollbar: bool,
    ) -> Self {
        let layout_style = match *theme.borrow() {
            Some(ref t) => {
                let mut style = Catalog::style(t, &class);
                style.sync_tab_appearance();
                style
            }
            None => {
                let mut style = crate::style::default(&iced::Theme::Dark);
                style.sync_tab_appearance();
                style
            }
        };
        let tabs_row =
            build_tabs_row(&layout_style, &tabs, active_tab, None, None, on_event.clone());
        Self {
            pane_id,
            tabs,
            active_tab,
            tabs_row,
            on_event,
            class,
            drag_threshold,
            drop_edge_fraction,
            hide_delay,
            show_scrollbar,
            theme,
        }
    }

    fn resolved_theme(&self) -> Option<Theme> {
        self.theme.borrow().clone()
    }

    fn layout_style_resolved(&self) -> DockStyle {
        match self.resolved_theme() {
            Some(ref t) => self.layout_style(t),
            None => {
                let mut style = crate::style::default(&iced::Theme::Dark);
                style.sync_tab_appearance();
                style
            }
        }
    }

    fn layout_style(&self, theme: &Theme) -> DockStyle {
        let mut style = Catalog::style(theme, &self.class);
        style.sync_tab_appearance();
        style
    }

    fn active_tab_index(&self) -> Option<usize> {
        self.tabs.iter().position(|tab| tab.id == self.active_tab)
    }

    fn rebuild_tabs_row(
        &mut self,
        theme: &Theme,
        hovered_tab: Option<NodeId>,
        pressed_tab: Option<NodeId>,
    ) {
        let style = self.layout_style(theme);
        self.tabs_row = build_tabs_row(
            &style,
            &self.tabs,
            self.active_tab,
            hovered_tab,
            pressed_tab,
            self.on_event.clone(),
        );
    }

    fn refresh_tabs_row(
        &mut self,
        tree: &mut Tree,
        theme: &Theme,
        hovered_tab: Option<NodeId>,
        pressed_tab: Option<NodeId>,
    ) {
        self.rebuild_tabs_row(theme, hovered_tab, pressed_tab);
        if !tree.children.is_empty() {
            tree.children[0].diff(&self.tabs_row);
        }
    }
}

fn visual_pressed_tab<Theme>(state: &TabStripState<Theme>) -> Option<NodeId> {
    if state.dragging {
        return None;
    }
    state.pressed_tab
}

fn tab_label_container_style(background: Color, border_radius: f32) -> container::Style {
    container::Style {
        background: if background.a > 0.0 {
            Some(iced::Background::Color(background))
        } else {
            None
        },
        border: Border {
            radius: border_radius.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

fn build_tabs_row<Message, Theme, Renderer>(
    style: &DockStyle,
    tabs: &[TabInfo],
    active_tab: NodeId,
    hovered_tab: Option<NodeId>,
    pressed_tab: Option<NodeId>,
    on_event: Rc<dyn Fn(DockAction) -> Message>,
) -> Element<'static, Message, Theme, Renderer>
where
    Message: Clone + 'static,
    Theme: Catalog
        + iced::widget::button::Catalog
        + iced::widget::container::Catalog
        + iced::widget::text::Catalog
        + 'static,
    Renderer: iced::advanced::Renderer + iced::advanced::text::Renderer + 'static,
    <Theme as iced::widget::button::Catalog>::Class<'static>:
        From<iced::widget::button::StyleFn<'static, Theme>>,
    <Theme as iced::widget::container::Catalog>::Class<'static>:
        From<iced::widget::container::StyleFn<'static, Theme>>,
    for<'a> <Theme as iced::widget::text::Catalog>::Class<'a>:
        From<iced::widget::text::StyleFn<'a, Theme>>,
{
    let bar = &style.tab_bar;
    let tab_style = &style.tab;
    let cb = &bar.close_button;
    let mut strip = row![]
        .spacing(bar.spacing)
        .padding(bar.padding)
        .width(Length::Shrink)
        .height(Length::Fixed(bar.height))
        .align_y(iced::Alignment::Center);
    for tab in tabs {
        let on_event = on_event.clone();
        let tab_id = tab.id;
        let is_active = tab.id == active_tab;
        let is_hovered = hovered_tab == Some(tab_id);
        let is_pressed = pressed_tab == Some(tab_id);
        let (label_bg, text_color) = if is_active {
            if is_pressed {
                (Color::TRANSPARENT, tab_style.pressed_text)
            } else {
                (Color::TRANSPARENT, tab_style.active_text)
            }
        } else if is_pressed {
            (tab_style.pressed_background, tab_style.pressed_text)
        } else if is_hovered {
            (tab_style.hovered_background, tab_style.hovered_text)
        } else {
            (tab_style.inactive_background, tab_style.inactive_text)
        };
        let border_radius = tab_style.border_radius;
        let label = container(text(tab.title.clone()).size(tab_style.text_size).color(text_color))
            .padding(Padding {
                top: tab_style.padding[0],
                bottom: tab_style.padding[0],
                left: tab_style.padding[1],
                right: tab_style.padding[1],
            })
            .height(Length::Fill)
            .center_y(Length::Fill);
        let close: Element<'_, Message, Theme, Renderer> = if tab.can_close {
            button(
                container(text(cb.label.clone()).size(cb.text_size))
                    .padding(Padding {
                        top: cb.padding[0],
                        bottom: cb.padding[0],
                        left: cb.padding[1],
                        right: cb.padding[1],
                    })
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(iced::Alignment::Center)
                    .align_y(iced::Alignment::Center),
            )
            .padding(Padding::ZERO)
            .width(Length::Fixed(cb.size))
            .height(Length::Fixed(cb.size))
            .style(close_button_style(cb))
                .on_press_with(move || {
                    (on_event)(DockAction::Tab(TabAction::Close { panel: tab_id }))
                })
                .into()
        } else {
            Space::new()
                .width(Length::Fixed(cb.size + cb.margin_right))
                .into()
        };
        let tab_row = row![
            label,
            close,
            Space::new().width(Length::Fixed(cb.margin_right))
        ]
        .height(Length::Fixed(bar.height))
        .align_y(iced::Alignment::Center);
        let tab_cell = mouse_area(
            container(tab_row).style(move |_: &Theme| tab_label_container_style(label_bg, border_radius)),
        );
        strip = strip.push(tab_cell);
    }
    strip.into()
}

fn hit_test_tab(
    row_layout: &Layout<'_>,
    tabs: &[TabInfo],
    scroll_offset: f32,
    pos: iced::Point,
) -> Option<NodeId> {
    let adjusted = iced::Point::new(pos.x + scroll_offset, pos.y);
    for (i, tab) in tabs.iter().enumerate() {
        let tab_layout = row_layout.children().nth(i)?;
        if tab_layout.bounds().contains(adjusted) {
            return Some(tab.id);
        }
    }
    None
}

fn close_button_bounds(tab_bounds: Rectangle, close: &CloseButtonStyle) -> Rectangle {
    Rectangle {
        x: tab_bounds.x + tab_bounds.width - close.margin_right - close.size,
        y: tab_bounds.y,
        width: close.size,
        height: tab_bounds.height,
    }
}

fn insert_marker_color(drop: &crate::style::DropOverlayStyle) -> Color {
    Color {
        a: drop.color.a.max(drop.insert_marker_min_alpha),
        ..drop.color
    }
}

/// Horizontal scroll offset of a tab strip widget tree.
pub(crate) fn scroll_offset<Theme: 'static>(tab_strip_tree: &Tree) -> f32 {
    tab_strip_tree
        .state
        .downcast_ref::<TabStripState<Theme>>()
        .scroll_offset
}

/// Layout-space X coordinates for each tab insertion slot.
///
/// Uses the same coordinate system as [`hit_test_tab`] (`cursor.x + scroll_offset`).
pub(crate) fn build_insert_x(row_layout: &Layout<'_>) -> Vec<f32> {
    let children: Vec<_> = row_layout.children().collect();
    if children.is_empty() {
        return vec![0.0];
    }

    let mut insert_x = Vec::with_capacity(children.len() + 1);
    for (i, child) in children.iter().enumerate() {
        let b = child.bounds();
        if i == 0 {
            insert_x.push(b.x);
        }
        if i + 1 < children.len() {
            let next = children[i + 1].bounds();
            insert_x.push((b.x + b.width + next.x) / 2.0);
        } else {
            insert_x.push(b.x + b.width);
        }
    }
    insert_x
}

/// Marker rectangle for a tab insertion slot in layout space (inside the scrolled tab row).
pub(crate) fn insert_marker_rect_layout(
    tab_bounds: Rectangle,
    insert_x: &[f32],
    index: usize,
    marker_width: f32,
) -> Option<Rectangle> {
    let layout_x = *insert_x.get(index)?;
    let width = marker_width.max(1.0);
    Some(Rectangle {
        x: layout_x - width / 2.0,
        y: tab_bounds.y,
        width,
        height: tab_bounds.height,
    })
}

/// Sync the insertion marker shown during an active tab drag.
pub(crate) fn set_insert_marker_index<Theme: 'static>(tree: &mut Tree, index: Option<usize>) -> bool {
    let state = tree.state.downcast_mut::<TabStripState<Theme>>();
    if state.insert_marker_index == index {
        return false;
    }
    state.insert_marker_index = index;
    true
}

/// Disable tab label / close-button hover while a tab drag is active anywhere in the dock.
pub(crate) fn set_suppress_hover<Theme: 'static>(tree: &mut Tree, suppress: bool) -> bool {
    let state = tree.state.downcast_mut::<TabStripState<Theme>>();
    if state.suppress_hover == suppress {
        return false;
    }
    state.suppress_hover = suppress;
    true
}

fn hit_test_close_button(
    row_layout: &Layout<'_>,
    tabs: &[TabInfo],
    scroll_offset: f32,
    pos: iced::Point,
    close: &CloseButtonStyle,
) -> bool {
    let adjusted = iced::Point::new(pos.x + scroll_offset, pos.y);
    for (i, tab) in tabs.iter().enumerate() {
        if !tab.can_close {
            continue;
        }
        let Some(tab_layout) = row_layout.children().nth(i) else {
            continue;
        };
        let bounds = close_button_bounds(tab_layout.bounds(), close);
        if bounds.contains(adjusted) {
            return true;
        }
    }
    false
}

fn max_scroll_offset(content_width: f32, viewport_width: f32) -> f32 {
    (content_width - viewport_width).max(0.0)
}

fn clamp_scroll_offset(offset: f32, max: f32) -> f32 {
    offset.clamp(0.0, max)
}

struct ScrollbarMetrics {
    track: Rectangle,
    thumb: Rectangle,
    max_offset: f32,
}

fn scrollbar_metrics(
    tab_bounds: Rectangle,
    bar: &crate::style::TabBarStyle,
    scroll_offset: f32,
    content_width: f32,
    viewport_width: f32,
) -> Option<ScrollbarMetrics> {
    let max_offset = max_scroll_offset(content_width, viewport_width);
    if max_offset <= 0.0 {
        return None;
    }

    let thumb_height = bar.scrollbar_height.max(1.0);
    let track = Rectangle {
        x: tab_bounds.x,
        y: tab_bounds.y + tab_bounds.height - thumb_height,
        width: tab_bounds.width,
        height: thumb_height,
    };

    let ratio = viewport_width / content_width;
    let thumb_width = (track.width * ratio).max(bar.scrollbar_thumb_min_width);
    let travel = (track.width - thumb_width).max(0.0);
    let thumb_x = if travel > 0.0 {
        track.x + (scroll_offset / max_offset) * travel
    } else {
        track.x
    };

    let thumb = Rectangle {
        x: thumb_x,
        y: track.y,
        width: thumb_width,
        height: thumb_height,
    };

    Some(ScrollbarMetrics {
        track,
        thumb,
        max_offset,
    })
}

fn draw_scrollbar<Renderer: iced::advanced::Renderer>(
    metrics: &ScrollbarMetrics,
    thumb_hovered: bool,
    bar: &crate::style::TabBarStyle,
    renderer: &mut Renderer,
) {
    let color = if thumb_hovered {
        bar.scrollbar_thumb_hovered
    } else {
        bar.scrollbar_thumb
    };
    if color.a <= 0.0 {
        return;
    }
    renderer.fill_quad(
        renderer::Quad {
            bounds: metrics.thumb,
            border: iced::Border {
                radius: (bar.scrollbar_height * 0.5).into(),
                ..iced::Border::default()
            },
            ..renderer::Quad::default()
        },
        color,
    );
}

/// Horizontal scroll delta for the tab row (vertical wheel scrolls horizontally).
fn scroll_delta_x(delta: mouse::ScrollDelta, shift: bool) -> f32 {
    const WHEEL_LINES: f32 = 60.0;
    match delta {
        mouse::ScrollDelta::Lines { x, y } => {
            let (x, y) = if cfg!(target_os = "macos") && shift {
                (y, x)
            } else {
                (x, y)
            };
            let movement = if !shift {
                Vector::new(x, y)
            } else {
                Vector::new(y, x)
            };
            -movement.y * WHEEL_LINES
        }
        mouse::ScrollDelta::Pixels { x, y } => -(x + y),
    }
}

fn cursor_over_tab_bar(tab_bounds: Rectangle, cursor: Cursor) -> bool {
    match cursor {
        Cursor::Available(point) | Cursor::Levitating(point) => tab_bounds.contains(point),
        Cursor::Unavailable => false,
    }
}

/// Whether this tab strip has an in-progress label drag (before or after threshold).
pub(crate) fn is_dragging<Theme: 'static>(tab_strip_tree: Option<&Tree>) -> bool {
    tab_strip_tree
        .map(|tree| {
            let state = tree.state.downcast_ref::<TabStripState<Theme>>();
            state.dragging || state.drag_pending
        })
        .unwrap_or(false)
}

/// Whether a tab label drag has passed the threshold (show grab cursor, etc.).
pub(crate) fn is_tab_drag_active<Theme: 'static>(tab_strip_tree: Option<&Tree>) -> bool {
    tab_strip_tree
        .map(|tree| tree.state.downcast_ref::<TabStripState<Theme>>().dragging)
        .unwrap_or(false)
}

/// Pending scrollbar hide deadline, if a delayed hide was scheduled.
pub(crate) fn pending_hide_deadline<Theme: 'static>(tree: &Tree) -> Option<iced::time::Instant> {
    tree.state
        .downcast_ref::<TabStripState<Theme>>()
        .hide_at
}

/// Ensure a redraw is scheduled when the hide deadline elapses.
pub(crate) fn schedule_hide_redraw<Message, Theme: 'static>(
    tree: &Tree,
    shell: &mut Shell<'_, Message>,
) {
    let Some(deadline) = pending_hide_deadline::<Theme>(tree) else {
        return;
    };

    if iced::time::Instant::now() >= deadline {
        Shell::replace_redraw_request(shell, window::RedrawRequest::NextFrame);
        return;
    }

    if shell.redraw_request() != window::RedrawRequest::NextFrame {
        Shell::replace_redraw_request(shell, window::RedrawRequest::At(deadline));
    }
}

fn tab_row_cursor(tab_bounds: Rectangle, cursor: Cursor, scroll_offset: f32) -> Cursor {
    if cursor_over_tab_bar(tab_bounds, cursor) {
        cursor + Vector::new(scroll_offset, 0.0)
    } else {
        Cursor::Unavailable
    }
}

/// Sync hover / hide state from a parent widget (e.g. [`TabDock`](crate::widget::tab_dock::TabDock)).
pub(crate) fn sync_hover_in_tree<Message, Theme: 'static>(
    tab_strip_tree: &mut Tree,
    tab_bounds: Rectangle,
    cursor: Cursor,
    hide_delay: iced::time::Duration,
    show_scrollbar: bool,
    shell: &mut Shell<'_, Message>,
) {
    let state = tab_strip_tree.state.downcast_mut::<TabStripState<Theme>>();
    sync_tab_bar_hover(state, tab_bounds, cursor, hide_delay, show_scrollbar, shell);
}

fn sync_tab_bar_hover<Message, Theme>(
    state: &mut TabStripState<Theme>,
    tab_bounds: Rectangle,
    cursor: Cursor,
    hide_delay: iced::time::Duration,
    show_scrollbar: bool,
    shell: &mut Shell<'_, Message>,
) {
    if !show_scrollbar {
        return;
    }

    if let Some(deadline) = state.hide_at {
        if iced::time::Instant::now() >= deadline {
            state.scrollbar_visible = false;
            state.hide_at = None;
        }
    }

    let over = cursor_over_tab_bar(tab_bounds, cursor);
    let was_hovered = state.tab_bar_hovered;

    if over {
        let changed = !state.tab_bar_hovered
            || !state.scrollbar_visible
            || state.hide_at.is_some();
        state.tab_bar_hovered = true;
        state.scrollbar_visible = true;
        state.hide_at = None;
        if changed {
            shell.request_redraw();
        }
    } else {
        if state.tab_bar_hovered {
            state.tab_bar_hovered = false;
        }
        if state.scrollbar_visible && state.hide_at.is_none() {
            state.hide_at = Some(iced::time::Instant::now() + hide_delay);
        }
        if was_hovered {
            shell.request_redraw();
        }
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for TabStrip<'_, Message, Theme, Renderer>
where
    Message: Clone + 'static,
    Theme: Catalog
        + iced::widget::button::Catalog
        + iced::widget::container::Catalog
        + iced::widget::text::Catalog
        + Clone
        + PartialEq
        + 'static,
    Renderer: iced::advanced::Renderer + iced::advanced::text::Renderer + 'static,
    <Theme as iced::widget::button::Catalog>::Class<'static>:
        From<iced::widget::button::StyleFn<'static, Theme>>,
    <Theme as iced::widget::container::Catalog>::Class<'static>:
        From<iced::widget::container::StyleFn<'static, Theme>>,
    for<'b> <Theme as iced::widget::text::Catalog>::Class<'b>:
        From<iced::widget::text::StyleFn<'b, Theme>>,
{
    fn tag(&self) -> Tag {
        Tag::of::<TabStripState<Theme>>()
    }

    fn state(&self) -> State {
        State::new(TabStripState::new(self.resolved_theme()))
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.tabs_row)]
    }

    fn diff(&self, tree: &mut Tree) {
        if tree.children.is_empty() {
            tree.children.push(Tree::new(&self.tabs_row));
            return;
        }
        tree.children[0].diff(&self.tabs_row);
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
        let style = self.layout_style_resolved();
        let bar_height = style.tab_bar.height;
        let max = limits.max();
        let viewport_width = max.width;
        let viewport_limits = layout::Limits::new(
            Size::ZERO,
            Size::new(f32::INFINITY, bar_height),
        );
        let row_node = compose::child_layout(
            &mut self.tabs_row,
            &mut tree.children[0],
            renderer,
            &viewport_limits,
        );
        let content_width = row_node.size().width;

        let state = tree.state.downcast_mut::<TabStripState<Theme>>();
        state.content_width = content_width;
        state.viewport_width = viewport_width;
        let max_offset = max_scroll_offset(content_width, viewport_width);
        state.scroll_offset = clamp_scroll_offset(state.scroll_offset, max_offset);

        layout::Node::with_children(
            Size::new(viewport_width, bar_height),
            vec![row_node],
        )
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
        let dock_style = self.layout_style(theme);
        let bar = &dock_style.tab_bar;
        let tab_style = &dock_style.tab;
        let drop_overlay = &dock_style.drop_overlay;
        let state = tree.state.downcast_ref::<TabStripState<Theme>>();
        let tab_bounds = layout.bounds();
        let Some(row_layout) = layout.children().next() else {
            return;
        };

        let scroll_offset = state.scroll_offset;
        let visible_bounds = tab_bounds.intersection(viewport).unwrap_or(tab_bounds);
        let overflow = state.content_width > state.viewport_width + f32::EPSILON;

        renderer.fill_quad(
            renderer::Quad {
                bounds: tab_bounds,
                ..renderer::Quad::default()
            },
            bar.background,
        );

        if let Some(sep) = &bar.separator {
            if sep.height > 0.0 && sep.color.a > 0.0 {
                let bounds = Rectangle {
                    x: tab_bounds.x,
                    y: tab_bounds.y + tab_bounds.height - sep.height,
                    width: tab_bounds.width,
                    height: sep.height,
                };
                renderer.fill_quad(
                    renderer::Quad {
                        bounds,
                        ..renderer::Quad::default()
                    },
                    sep.color,
                );
            }
        }

        let translation = Vector::new(scroll_offset, 0.0);

        renderer.with_layer(visible_bounds, |renderer| {
            renderer.with_translation(Vector::new(-translation.x, -translation.y), |renderer| {
                if let Some(active_i) = self.active_tab_index() {
                    if let Some(active_layout) = row_layout.children().nth(active_i) {
                        let btn_bounds = active_layout.bounds();
                        let fill = Rectangle {
                            x: btn_bounds.x,
                            y: btn_bounds.y,
                            width: btn_bounds.width,
                            height: (tab_bounds.height - btn_bounds.y + tab_bounds.y + 1.0).max(0.0),
                        };
                        renderer.fill_quad(
                            renderer::Quad {
                                bounds: fill,
                                ..renderer::Quad::default()
                            },
                            dock_style.tab.active_background,
                        );
                        let accent_h = tab_style.active_accent_height.max(0.0);
                        if accent_h > 0.0 {
                            let accent_y = tab_bounds.y + tab_bounds.height - accent_h;
                            renderer.fill_quad(
                                renderer::Quad {
                                    bounds: Rectangle {
                                        x: btn_bounds.x,
                                        y: accent_y,
                                        width: btn_bounds.width,
                                        height: accent_h,
                                    },
                                    ..renderer::Quad::default()
                                },
                                tab_style.active_accent,
                            );
                        }
                    }
                }
                let content_cursor =
                    tab_row_cursor(tab_bounds, cursor, translation.x);
                compose::child_draw(
                    &self.tabs_row,
                    &tree.children[0],
                    renderer,
                    theme,
                    style,
                    row_layout,
                    content_cursor,
                    &Rectangle {
                        x: visible_bounds.x + translation.x,
                        y: visible_bounds.y,
                        ..visible_bounds
                    },
                );

                if let Some(index) = state.insert_marker_index {
                    let insert_x = build_insert_x(&row_layout);
                    if let Some(marker) = insert_marker_rect_layout(
                        tab_bounds,
                        &insert_x,
                        index,
                        drop_overlay.insert_marker_width,
                    ) {
                        renderer.fill_quad(
                            renderer::Quad {
                                bounds: marker,
                                ..renderer::Quad::default()
                            },
                            insert_marker_color(drop_overlay),
                        );
                    }
                }
            });

            let scrollbar_fade_in = state.scrollbar_visible
                && state.hide_at.is_none_or(|deadline| iced::time::Instant::now() < deadline);

            if self.show_scrollbar && overflow && scrollbar_fade_in {
                if let Some(metrics) = scrollbar_metrics(
                    tab_bounds,
                    bar,
                    scroll_offset,
                    state.content_width,
                    state.viewport_width,
                ) {
                    let thumb_hovered = cursor
                        .position()
                        .is_some_and(|p| metrics.thumb.contains(p));
                    draw_scrollbar(
                        &metrics,
                        thumb_hovered || state.scrollbar_drag.is_some(),
                        bar,
                        renderer,
                    );
                }
            }
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
        let tab_bounds = layout.bounds();
        let current_theme = self.resolved_theme();
        let dock_style = self.layout_style_resolved();
        let bar = &dock_style.tab_bar;
        let threshold = self.drag_threshold;
        let cursor_pos = cursor.position();
        let over_tab_bar = cursor_over_tab_bar(tab_bounds, cursor);
        let mut row_refresh: Option<(Option<NodeId>, Option<NodeId>)> = None;

        {
        let state = tree.state.downcast_mut::<TabStripState<Theme>>();
        let max_offset = max_scroll_offset(state.content_width, state.viewport_width);

        if state.built_theme != current_theme {
            state.built_theme = current_theme.clone();
            row_refresh = Some((state.hovered_tab, visual_pressed_tab(state)));
        }

        if state.suppress_hover && state.hovered_tab.is_some() {
            state.hovered_tab = None;
            row_refresh = Some((None, visual_pressed_tab(state)));
            shell.request_redraw();
        }

        let hovered = if state.suppress_hover {
            None
        } else if over_tab_bar {
            cursor_pos.and_then(|pos| {
                layout.children().next().and_then(|row_layout| {
                    hit_test_tab(
                        &row_layout,
                        &self.tabs,
                        state.scroll_offset,
                        pos,
                    )
                })
            })
        } else {
            None
        };
        if hovered != state.hovered_tab {
            state.hovered_tab = hovered;
            row_refresh = Some((hovered, visual_pressed_tab(state)));
            shell.request_redraw();
        }

        if self.show_scrollbar {
            if let Event::Window(window::Event::RedrawRequested(now)) = event {
                if let Some(deadline) = state.hide_at {
                    if *now >= deadline {
                        state.scrollbar_visible = false;
                        state.hide_at = None;
                    } else {
                        Shell::replace_redraw_request(shell, window::RedrawRequest::At(deadline));
                    }
                }
            }
        }

        if let Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) = event {
            state.keyboard_modifiers = *modifiers;
        }

        let scrollbar_metrics = scrollbar_metrics(
            tab_bounds,
            bar,
            state.scroll_offset,
            state.content_width,
            state.viewport_width,
        );

        let mut captured_scrollbar = false;
        let mut captured_wheel = false;
        let mut captured_label = false;

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if self.show_scrollbar {
                    if let (Some(pos), Some(metrics)) = (cursor_pos, scrollbar_metrics.as_ref()) {
                        if metrics.thumb.contains(pos) {
                            state.scrollbar_drag = Some(ScrollbarDrag {
                                grab_x: pos.x - metrics.thumb.x,
                            });
                            state.scrollbar_visible = true;
                            state.hide_at = None;
                            shell.capture_event();
                            captured_scrollbar = true;
                        } else if metrics.track.contains(pos) {
                            let click = (pos.x - metrics.track.x - metrics.thumb.width * 0.5)
                                .max(0.0);
                            let travel = (metrics.track.width - metrics.thumb.width).max(0.0);
                            if travel > 0.0 {
                                state.scroll_offset = clamp_scroll_offset(
                                    (click / travel) * metrics.max_offset,
                                    metrics.max_offset,
                                );
                            }
                            state.scrollbar_visible = true;
                            state.hide_at = None;
                            shell.capture_event();
                            shell.request_redraw();
                            captured_scrollbar = true;
                        }
                    }
                }
                if !captured_scrollbar {
                    if let (Some(pos), Some(row_layout)) =
                        (cursor_pos, layout.children().next())
                    {
                        let on_close = hit_test_close_button(
                            &row_layout,
                            &self.tabs,
                            state.scroll_offset,
                            pos,
                            &bar.close_button,
                        );
                        if !on_close {
                            if let Some(tab_id) = hit_test_tab(
                                &row_layout,
                                &self.tabs,
                                state.scroll_offset,
                                pos,
                            ) {
                                let can_drag = self
                                    .tabs
                                    .iter()
                                    .find(|t| t.id == tab_id)
                                    .is_some_and(|t| t.can_drag);
                                state.pressed_tab = Some(tab_id);
                                if can_drag {
                                    state.drag_pending = true;
                                    state.drag_start = Some(pos);
                                }
                                    row_refresh = Some((state.hovered_tab, Some(tab_id)));
                                    shell.request_redraw();
                                captured_label = true;
                            }
                        }
                    }
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if self.show_scrollbar {
                    if let (Some(pos), Some(drag), Some(metrics)) = (
                        cursor_pos,
                        state.scrollbar_drag,
                        scrollbar_metrics.as_ref(),
                    ) {
                        let travel = (metrics.track.width - metrics.thumb.width).max(0.0);
                        if travel > 0.0 {
                            let delta = pos.x - metrics.track.x - drag.grab_x;
                            let ratio = delta / travel;
                            state.scroll_offset = clamp_scroll_offset(
                                ratio * metrics.max_offset,
                                metrics.max_offset,
                            );
                            shell.capture_event();
                            shell.request_redraw();
                            captured_scrollbar = true;
                        }
                    }
                }
                if state.drag_pending {
                    if let (Some(start), Some(pos)) = (state.drag_start, cursor_pos) {
                        let dx = pos.x - start.x;
                        let dy = pos.y - start.y;
                        if (dx * dx + dy * dy).sqrt() >= threshold {
                            if let Some(panel) = state.pressed_tab {
                                state.dragging = true;
                                state.drag_pending = false;
                                row_refresh = Some((state.hovered_tab, None));
                                let drop_edge_fraction = self.drop_edge_fraction;
                                shell.publish((self.on_event)(DockAction::Tab(
                                    TabAction::DragStarted {
                                        source_pane: self.pane_id,
                                        source_panel: panel,
                                        drop_edge_fraction,
                                    },
                                )));
                                shell.request_redraw();
                                captured_label = true;
                            }
                        }
                    }
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if self.show_scrollbar && state.scrollbar_drag.take().is_some() {
                    shell.request_redraw();
                }
                let was_dragging = state.dragging;
                let pressed = state.pressed_tab.take();
                state.drag_pending = false;
                state.drag_start = None;
                state.dragging = false;

                if was_dragging {
                    row_refresh = Some((state.hovered_tab, None));
                    shell.request_redraw();
                    captured_label = true;
                } else if let Some(panel) = pressed {
                    row_refresh = Some((state.hovered_tab, None));
                    shell.publish((self.on_event)(DockAction::Tab(TabAction::Select {
                        pane: self.pane_id,
                        panel,
                    })));
                    captured_label = true;
                }
            }
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                if over_tab_bar && max_offset > 0.0 {
                    let shift = state.keyboard_modifiers.shift();
                    let dx = scroll_delta_x(*delta, shift);
                    if dx.abs() > f32::EPSILON {
                        state.scroll_offset =
                            clamp_scroll_offset(state.scroll_offset + dx, max_offset);
                        if self.show_scrollbar {
                            state.scrollbar_visible = true;
                            state.hide_at = None;
                        }
                        shell.capture_event();
                        shell.request_redraw();
                        captured_wheel = true;
                    }
                }
            }
            _ => {}
        }

        if self.show_scrollbar {
            if let Some(metrics) = scrollbar_metrics.as_ref() {
                let hovered = cursor_pos.is_some_and(|p| metrics.thumb.contains(p));
                if hovered != state.scrollbar_thumb_hovered {
                    state.scrollbar_thumb_hovered = hovered;
                    shell.request_redraw();
                }
            }
        }

        let forward_wheel = !matches!(
            event,
            Event::Mouse(mouse::Event::WheelScrolled { .. })
        );

        if !captured_scrollbar && !captured_label && (forward_wheel || !captured_wheel) {
            if let Some(row_layout) = layout.children().next() {
                let content_cursor = if state.suppress_hover {
                    Cursor::Unavailable
                } else {
                    tab_row_cursor(tab_bounds, cursor, state.scroll_offset)
                };
                compose::child_update(
                    &mut self.tabs_row,
                    &mut tree.children[0],
                    event,
                    row_layout,
                    content_cursor,
                    renderer,
                    clipboard,
                    shell,
                    viewport,
                );
            }
        }

        sync_tab_bar_hover(
            state,
            tab_bounds,
            cursor,
            self.hide_delay,
            self.show_scrollbar,
            shell,
        );
        }

        if let Some((hovered, pressed)) = row_refresh {
            if let Some(ref t) = current_theme {
                self.refresh_tabs_row(tree, t, hovered, pressed);
            }
            tree.state
                .downcast_mut::<TabStripState<Theme>>()
                .built_theme = current_theme;
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
        let state = tree.state.downcast_ref::<TabStripState<Theme>>();
        if state.dragging {
            return mouse::Interaction::Grab;
        }
        if state.suppress_hover {
            return mouse::Interaction::Grab;
        }
        if self.show_scrollbar && state.scrollbar_drag.is_some() {
            return mouse::Interaction::Grabbing;
        }

        let tab_bounds = layout.bounds();
        let dock_style = self.layout_style_resolved();
        if self.show_scrollbar {
            if let Some(metrics) = scrollbar_metrics(
                tab_bounds,
                &dock_style.tab_bar,
                state.scroll_offset,
                state.content_width,
                state.viewport_width,
            ) {
                if cursor.position().is_some_and(|p| {
                    metrics.thumb.contains(p) || metrics.track.contains(p)
                }) {
                    return mouse::Interaction::Pointer;
                }
            }
        }

        let content_cursor =
            tab_row_cursor(tab_bounds, cursor, state.scroll_offset);
        if let Some(row_layout) = layout.children().next() {
            return compose::child_mouse_interaction(
                &self.tabs_row,
                &tree.children[0],
                row_layout,
                content_cursor,
                viewport,
                renderer,
            );
        }
        mouse::Interaction::default()
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        if let Some(row_layout) = layout.children().next() {
            compose::child_operate(
                &mut self.tabs_row,
                &mut tree.children[0],
                row_layout,
                renderer,
                operation,
            );
        }
    }
}

impl<'a, Message, Theme, Renderer> From<TabStrip<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: Clone + 'static,
    Theme: Catalog
        + iced::widget::button::Catalog
        + iced::widget::container::Catalog
        + iced::widget::text::Catalog
        + Clone
        + PartialEq
        + 'static,
    Renderer: iced::advanced::Renderer + iced::advanced::text::Renderer + 'static,
    <Theme as iced::widget::button::Catalog>::Class<'static>:
        From<iced::widget::button::StyleFn<'static, Theme>>,
    <Theme as iced::widget::container::Catalog>::Class<'static>:
        From<iced::widget::container::StyleFn<'static, Theme>>,
    for<'b> <Theme as iced::widget::text::Catalog>::Class<'b>:
        From<iced::widget::text::StyleFn<'b, Theme>>,
{
    fn from(widget: TabStrip<'a, Message, Theme, Renderer>) -> Self {
        Element::new(widget)
    }
}
