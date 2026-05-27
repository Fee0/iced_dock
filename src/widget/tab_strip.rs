use std::cell::RefCell;
use std::rc::Rc;

use crate::model::NodeId;
use crate::style::{
    self, close_button_style, Catalog, DockStyle, DropOverlayStyle, TabBarStyle,
};
use crate::widget::action::{DockAction, TabAction};
use crate::widget::compose;
use crate::widget::dock::TabBarScrollbarAttachment;
use crate::widget::tab_dock::TabInfo;
use iced::animation::Animation;
use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::tree::{State, Tag, Tree};
use iced::advanced::widget::{Operation, Widget};
use iced::advanced::{self, Clipboard, Shell};
use iced::keyboard;
use iced::mouse::{self, Cursor};
use iced::time::{Duration, Instant};
use iced::widget::overlay::menu;
use iced::widget::{button, container, mouse_area, row, text, Space};
use iced::window;
use iced::Theme as IcedTheme;
use iced::{Border, Color, Element, Event, Length, Padding, Rectangle, Size, Vector};
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

#[derive(Debug, Clone, Copy)]
struct ScrollbarDrag {
    grab_x: f32,
}

#[derive(Debug, Clone)]
struct HiddenTabOption {
    id: NodeId,
    title: String,
}

impl Display for HiddenTabOption {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.write_str(&self.title)
    }
}

#[derive(Debug, Default)]
struct OverflowUiState {
    open: bool,
    pressed: bool,
    reveal_tab: Option<NodeId>,
}

struct TabStripState<Theme: menu::Catalog> {
    scroll_offset: f32,
    content_width: f32,
    viewport_width: f32,
    show_overflow_button: bool,
    tab_bar_hovered: bool,
    scrollbar_visibility: Animation<bool>,
    scrollbar_fade_duration: Duration,
    scrollbar_animated: bool,
    scrollbar_drag: Option<ScrollbarDrag>,
    scrollbar_thumb_hovered: bool,
    overflow_button_hovered: bool,
    overflow_menu_state: menu::State,
    overflow_menu_hovered: Option<usize>,
    overflow_options: Vec<HiddenTabOption>,
    overflow_menu_class: <Theme as menu::Catalog>::Class<'static>,
    overflow_ui: Rc<RefCell<OverflowUiState>>,
    keyboard_modifiers: keyboard::Modifiers,
    drag_pending: bool,
    drag_start: Option<iced::Point>,
    dragging: bool,
    pressed_tab: Option<NodeId>,
    hovered_tab: Option<NodeId>,
    insert_marker_index: Option<usize>,
    drag_blocked: bool,
    /// When true, tab label and close-button hover are disabled (active global tab drag).
    suppress_hover: bool,
    /// Theme used for the last [`build_tabs_row`] rebuild.
    built_theme: Option<Theme>,
}

impl<Theme> TabStripState<Theme>
where
    Theme: menu::Catalog,
{
    fn new(
        theme: Option<Theme>,
        scrollbar_fade_duration: Duration,
        scrollbar_animated: bool,
    ) -> Self {
        Self {
            scroll_offset: 0.0,
            content_width: 0.0,
            viewport_width: 0.0,
            show_overflow_button: false,
            tab_bar_hovered: false,
            scrollbar_visibility: Animation::new(false).duration(scrollbar_fade_duration),
            scrollbar_fade_duration,
            scrollbar_animated,
            scrollbar_drag: None,
            scrollbar_thumb_hovered: false,
            overflow_button_hovered: false,
            overflow_menu_state: menu::State::default(),
            overflow_menu_hovered: None,
            overflow_options: Vec::new(),
            overflow_menu_class: <Theme as menu::Catalog>::default(),
            overflow_ui: Rc::new(RefCell::new(OverflowUiState::default())),
            keyboard_modifiers: keyboard::Modifiers::default(),
            drag_pending: false,
            drag_start: None,
            dragging: false,
            pressed_tab: None,
            hovered_tab: None,
            insert_marker_index: None,
            drag_blocked: false,
            suppress_hover: false,
            built_theme: theme,
        }
    }
}

const OVERFLOW_BUTTON_HORIZONTAL_PADDING: f32 = 8.0;
const OVERFLOW_BUTTON_CONTENT_WIDTH: f32 = 16.0;
const OVERFLOW_MENU_PADDING: Padding = Padding {
    top: 6.0,
    right: 10.0,
    bottom: 6.0,
    left: 10.0,
};
const OVERFLOW_BUTTON_TOTAL_WIDTH: f32 =
    OVERFLOW_BUTTON_CONTENT_WIDTH + 2.0 * OVERFLOW_BUTTON_HORIZONTAL_PADDING;

struct OverflowMenuOverlay<'a, Message, Theme, Renderer>
where
    Renderer: advanced::Renderer,
{
    menu: overlay::Element<'a, Message, Theme, Renderer>,
    button_bounds: Rectangle,
    ui: Rc<RefCell<OverflowUiState>>,
}

impl<Message, Theme, Renderer> advanced::Overlay<Message, Theme, Renderer>
    for OverflowMenuOverlay<'_, Message, Theme, Renderer>
where
    Renderer: advanced::Renderer,
{
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        self.menu.as_overlay_mut().layout(renderer, bounds)
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: Cursor,
    ) {
        self.menu
            .as_overlay()
            .draw(renderer, theme, style, layout, cursor);
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        if matches!(
            event,
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
        ) {
            if let Some(position) = cursor.position() {
                if !layout.bounds().contains(position) && !self.button_bounds.contains(position) {
                    let mut ui = self.ui.borrow_mut();
                    ui.open = false;
                    ui.pressed = false;
                    shell.request_redraw();
                }
            }
        }

        self.menu
            .as_overlay_mut()
            .update(event, layout, cursor, renderer, clipboard, shell);
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: Cursor,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.menu
            .as_overlay()
            .mouse_interaction(layout, cursor, renderer)
    }

    fn operate(&mut self, layout: Layout<'_>, renderer: &Renderer, operation: &mut dyn Operation) {
        self.menu
            .as_overlay_mut()
            .operate(layout, renderer, operation);
    }

    fn overlay<'b>(
        &'b mut self,
        layout: Layout<'b>,
        renderer: &Renderer,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.menu.as_overlay_mut().overlay(layout, renderer)
    }

    fn index(&self) -> f32 {
        10.0
    }
}

pub struct TabStrip<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Theme: Catalog,
    Renderer: advanced::Renderer,
{
    pane_id: NodeId,
    tabs: Vec<TabInfo>,
    active_tab: NodeId,
    tabs_row: Element<'a, Message, Theme, Renderer>,
    on_event: Rc<dyn Fn(DockAction) -> Message>,
    class: Rc<<Theme as Catalog>::Class<'static>>,
    tab_bar_height: f32,
    tab_bar_spacing: f32,
    tab_bar_padding: [f32; 2],
    tab_text_size: f32,
    tab_padding: [f32; 2],
    tab_accent_height: f32,
    close_button_text_size: f32,
    close_button_size: f32,
    close_button_margin_right: f32,
    close_button_padding: [f32; 2],
    scrollbar_height: f32,
    scrollbar_thumb_min_width: f32,
    insert_marker_width: f32,
    separator_height: f32,
    drag_threshold: f32,
    drop_edge_fraction: f32,
    scrollbar_fade_duration: Duration,
    scrollbar_animated: bool,
    show_scrollbar: bool,
    scrollbar_attachment: TabBarScrollbarAttachment,
    theme: Rc<RefCell<Option<Theme>>>,
}

impl<Message, Theme, Renderer> TabStrip<'_, Message, Theme, Renderer>
where
    Message: Clone + 'static,
    Theme: Catalog
        + button::Catalog
        + container::Catalog
        + menu::Catalog
        + text::Catalog
        + Clone
        + PartialEq
        + 'static,
    Renderer: advanced::Renderer + advanced::text::Renderer + 'static,
    <Theme as button::Catalog>::Class<'static>: From<button::StyleFn<'static, Theme>>,
    <Theme as container::Catalog>::Class<'static>: From<container::StyleFn<'static, Theme>>,
    for<'b> <Theme as text::Catalog>::Class<'b>: From<text::StyleFn<'b, Theme>>,
{
    pub(crate) fn new(
        pane_id: NodeId,
        tabs: Vec<TabInfo>,
        active_tab: NodeId,
        on_event: Rc<dyn Fn(DockAction) -> Message>,
        class: Rc<<Theme as Catalog>::Class<'static>>,
        theme: Rc<RefCell<Option<Theme>>>,
        tab_bar_height: f32,
        tab_bar_spacing: f32,
        tab_bar_padding: [f32; 2],
        tab_text_size: f32,
        tab_padding: [f32; 2],
        tab_accent_height: f32,
        close_button_text_size: f32,
        close_button_size: f32,
        close_button_margin_right: f32,
        close_button_padding: [f32; 2],
        scrollbar_height: f32,
        scrollbar_thumb_min_width: f32,
        insert_marker_width: f32,
        separator_height: f32,
        drag_threshold: f32,
        drop_edge_fraction: f32,
        scrollbar_fade_duration: Duration,
        scrollbar_animated: bool,
        show_scrollbar: bool,
        scrollbar_attachment: TabBarScrollbarAttachment,
    ) -> Self {
        let paint_style = match &*theme.borrow() {
            Some(t) => {
                let mut style = Catalog::style(t, &class);
                style.sync_tab_appearance();
                style
            }
            None => {
                let mut style = style::default(&IcedTheme::Dark);
                style.sync_tab_appearance();
                style
            }
        };
        let tabs_row = build_tabs_row(
            &paint_style,
            tab_bar_height,
            tab_bar_spacing,
            tab_bar_padding,
            tab_text_size,
            tab_padding,
            close_button_text_size,
            close_button_size,
            close_button_margin_right,
            close_button_padding,
            &tabs,
            active_tab,
            None,
            None,
            Rc::clone(&on_event),
        );
        Self {
            pane_id,
            tabs,
            active_tab,
            tabs_row,
            on_event,
            class,
            tab_bar_height,
            tab_bar_spacing,
            tab_bar_padding,
            tab_text_size,
            tab_padding,
            tab_accent_height,
            close_button_text_size,
            close_button_size,
            close_button_margin_right,
            close_button_padding,
            scrollbar_height,
            scrollbar_thumb_min_width,
            insert_marker_width,
            separator_height,
            drag_threshold,
            drop_edge_fraction,
            scrollbar_fade_duration,
            scrollbar_animated,
            show_scrollbar,
            scrollbar_attachment,
            theme,
        }
    }

    fn resolved_theme(&self) -> Option<Theme> {
        self.theme.borrow().clone()
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
            self.tab_bar_height,
            self.tab_bar_spacing,
            self.tab_bar_padding,
            self.tab_text_size,
            self.tab_padding,
            self.close_button_text_size,
            self.close_button_size,
            self.close_button_margin_right,
            self.close_button_padding,
            &self.tabs,
            self.active_tab,
            hovered_tab,
            pressed_tab,
            Rc::clone(&self.on_event),
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

fn visual_pressed_tab<Theme: menu::Catalog>(state: &TabStripState<Theme>) -> Option<NodeId> {
    if state.dragging {
        return None;
    }
    state.pressed_tab
}

fn tab_label_container_style(background: Color, border_radius: f32) -> container::Style {
    container::Style {
        background: (background.a > 0.0).then_some(iced::Background::Color(background)),
        border: Border {
            radius: border_radius.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

fn build_tabs_row<Message, Theme, Renderer>(
    style: &DockStyle,
    tab_bar_height: f32,
    tab_bar_spacing: f32,
    tab_bar_padding: [f32; 2],
    tab_text_size: f32,
    tab_padding: [f32; 2],
    close_button_text_size: f32,
    close_button_size: f32,
    close_button_margin_right: f32,
    close_button_padding: [f32; 2],
    tabs: &[TabInfo],
    active_tab: NodeId,
    hovered_tab: Option<NodeId>,
    pressed_tab: Option<NodeId>,
    on_event: Rc<dyn Fn(DockAction) -> Message>,
) -> Element<'static, Message, Theme, Renderer>
where
    Message: Clone + 'static,
    Theme: Catalog + button::Catalog + container::Catalog + text::Catalog + 'static,
    Renderer: advanced::Renderer + advanced::text::Renderer + 'static,
    <Theme as button::Catalog>::Class<'static>: From<button::StyleFn<'static, Theme>>,
    <Theme as container::Catalog>::Class<'static>: From<container::StyleFn<'static, Theme>>,
    for<'a> <Theme as text::Catalog>::Class<'a>: From<text::StyleFn<'a, Theme>>,
{
    let tab_style = &style.tab;
    let cb = &style.tab_bar.close_button;
    let mut strip = row![]
        .spacing(tab_bar_spacing)
        .padding(tab_bar_padding)
        .width(Length::Shrink)
        .height(Length::Fixed(tab_bar_height))
        .align_y(iced::Alignment::Center);
    for tab in tabs {
        let on_event = Rc::clone(&on_event);
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
        let label = container(
            text(tab.title.clone())
                .size(tab_text_size)
                .color(text_color),
        )
        .padding(Padding {
            top: tab_padding[0],
            bottom: tab_padding[0],
            left: tab_padding[1],
            right: tab_padding[1],
        })
        .height(Length::Fill)
        .center_y(Length::Fill);
        let close: Element<'_, Message, Theme, Renderer> = if tab.can_close {
            button(
                container(text(cb.label.clone()).size(close_button_text_size))
                    .padding(Padding {
                        top: close_button_padding[0],
                        bottom: close_button_padding[0],
                        left: close_button_padding[1],
                        right: close_button_padding[1],
                    })
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(iced::Alignment::Center)
                    .align_y(iced::Alignment::Center),
            )
            .padding(Padding::ZERO)
            .width(Length::Fixed(close_button_size))
            .height(Length::Fixed(close_button_size))
            .style(close_button_style(cb))
            .on_press_with(move || (on_event)(DockAction::Tab(TabAction::Close { panel: tab_id })))
            .into()
        } else {
            Space::new()
                .width(Length::Fixed(close_button_size + close_button_margin_right))
                .into()
        };
        let tab_row = row![
            label,
            close,
            Space::new().width(Length::Fixed(close_button_margin_right))
        ]
        .height(Length::Fixed(tab_bar_height))
        .align_y(iced::Alignment::Center);
        let tab_cell = mouse_area(
            container(tab_row)
                .style(move |_: &Theme| tab_label_container_style(label_bg, border_radius)),
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

fn close_button_bounds(tab_bounds: Rectangle, close_size: f32, close_margin_right: f32) -> Rectangle {
    Rectangle {
        x: tab_bounds.x + tab_bounds.width - close_margin_right - close_size,
        y: tab_bounds.y,
        width: close_size,
        height: tab_bounds.height,
    }
}

fn insert_marker_color(drop: &DropOverlayStyle, blocked: bool) -> Color {
    let base = if blocked {
        drop.blocked_color
    } else {
        drop.color
    };
    Color {
        a: base.a.max(drop.insert_marker_min_alpha),
        ..base
    }
}

/// Horizontal scroll offset of a tab strip widget tree.
pub(crate) fn scroll_offset<Theme: menu::Catalog + 'static>(tab_strip_tree: &Tree) -> f32 {
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
pub(crate) fn set_insert_marker_index<Theme: menu::Catalog + 'static>(
    tree: &mut Tree,
    index: Option<usize>,
) -> bool {
    let state = tree.state.downcast_mut::<TabStripState<Theme>>();
    if state.insert_marker_index == index {
        return false;
    }
    state.insert_marker_index = index;
    true
}

/// Mark the insertion marker as blocked (group mismatch) so it renders in a different color.
pub(crate) fn set_drag_blocked<Theme: menu::Catalog + 'static>(
    tree: &mut Tree,
    blocked: bool,
) -> bool {
    let state = tree.state.downcast_mut::<TabStripState<Theme>>();
    if state.drag_blocked == blocked {
        return false;
    }
    state.drag_blocked = blocked;
    true
}

/// Disable tab label / close-button hover while a tab drag is active anywhere in the dock.
pub(crate) fn set_suppress_hover<Theme: menu::Catalog + 'static>(
    tree: &mut Tree,
    suppress: bool,
) -> bool {
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
    close_size: f32,
    close_margin_right: f32,
) -> bool {
    let adjusted = iced::Point::new(pos.x + scroll_offset, pos.y);
    for (i, tab) in tabs.iter().enumerate() {
        if !tab.can_close {
            continue;
        }
        let Some(tab_layout) = row_layout.children().nth(i) else {
            continue;
        };
        let bounds = close_button_bounds(tab_layout.bounds(), close_size, close_margin_right);
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

fn overflow_button_bounds(tab_bounds: Rectangle, viewport_width: f32) -> Rectangle {
    Rectangle {
        x: tab_bounds.x + viewport_width,
        y: tab_bounds.y,
        width: (tab_bounds.width - viewport_width).max(0.0),
        height: tab_bounds.height,
    }
}

fn visible_row_bounds(tab_bounds: Rectangle, viewport_width: f32) -> Rectangle {
    Rectangle {
        width: viewport_width.max(0.0),
        ..tab_bounds
    }
}

fn tab_visible_range(tab_x: f32, scroll_offset: f32, viewport_width: f32) -> (f32, f32) {
    let start = tab_x + scroll_offset;
    (start, start + viewport_width)
}

fn tab_fully_visible(
    tab_bounds: Rectangle,
    row_x: f32,
    scroll_offset: f32,
    viewport_width: f32,
) -> bool {
    let (visible_start, visible_end) = tab_visible_range(row_x, scroll_offset, viewport_width);
    tab_bounds.x >= visible_start && tab_bounds.x + tab_bounds.width <= visible_end
}

fn hidden_tabs(
    row_layout: &Layout<'_>,
    tabs: &[TabInfo],
    row_x: f32,
    scroll_offset: f32,
    viewport_width: f32,
) -> Vec<HiddenTabOption> {
    tabs.iter()
        .enumerate()
        .filter_map(|(index, tab)| {
            let tab_bounds = row_layout.children().nth(index)?.bounds();
            (!tab_fully_visible(tab_bounds, row_x, scroll_offset, viewport_width)).then(|| {
                HiddenTabOption {
                    id: tab.id,
                    title: tab.title.clone(),
                }
            })
        })
        .collect()
}

fn ensure_tab_visible(
    tab_bounds: Rectangle,
    row_x: f32,
    scroll_offset: f32,
    viewport_width: f32,
    max_offset: f32,
) -> f32 {
    let (visible_start, visible_end) = tab_visible_range(row_x, scroll_offset, viewport_width);
    if tab_bounds.x < visible_start {
        clamp_scroll_offset(tab_bounds.x - row_x, max_offset)
    } else if tab_bounds.x + tab_bounds.width > visible_end {
        clamp_scroll_offset(
            tab_bounds.x + tab_bounds.width - row_x - viewport_width,
            max_offset,
        )
    } else {
        scroll_offset
    }
}

struct ScrollbarMetrics {
    track: Rectangle,
    thumb: Rectangle,
    max_offset: f32,
}

fn scrollbar_metrics(
    tab_bounds: Rectangle,
    scrollbar_height: f32,
    scrollbar_attachment: TabBarScrollbarAttachment,
    scrollbar_thumb_min_width: f32,
    scroll_offset: f32,
    content_width: f32,
    viewport_width: f32,
) -> Option<ScrollbarMetrics> {
    let max_offset = max_scroll_offset(content_width, viewport_width);
    if max_offset <= 0.0 {
        return None;
    }

    let thumb_height = scrollbar_height.max(1.0);
    let track = Rectangle {
        x: tab_bounds.x,
        y: match scrollbar_attachment {
            TabBarScrollbarAttachment::Top => tab_bounds.y,
            TabBarScrollbarAttachment::Bottom => {
                tab_bounds.y + (tab_bounds.height - thumb_height).max(0.0)
            }
        },
        width: tab_bounds.width,
        height: thumb_height,
    };

    let ratio = viewport_width / content_width;
    let thumb_width = (track.width * ratio).max(scrollbar_thumb_min_width);
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

fn draw_scrollbar<Renderer: advanced::Renderer>(
    metrics: &ScrollbarMetrics,
    thumb_hovered: bool,
    bar: &TabBarStyle,
    scrollbar_height: f32,
    alpha: f32,
    renderer: &mut Renderer,
) {
    if alpha <= 0.0 {
        return;
    }

    let thumb_color = if thumb_hovered {
        bar.scrollbar_thumb_hovered
    } else {
        bar.scrollbar_thumb
    }
    .scale_alpha(alpha);
    let track_color = bar.scrollbar_track.scale_alpha(alpha);
    let thumb_border = bar.scrollbar_thumb_border.scale_alpha(alpha);

    if thumb_color.a <= 0.0 && track_color.a <= 0.0 {
        return;
    }

    renderer.fill_quad(
        renderer::Quad {
            bounds: metrics.track,
            border: iced::Border {
                radius: (scrollbar_height * 0.5).into(),
                ..iced::Border::default()
            },
            ..renderer::Quad::default()
        },
        track_color,
    );
    renderer.fill_quad(
        renderer::Quad {
            bounds: metrics.thumb,
            border: iced::Border {
                width: 1.0,
                color: thumb_border,
                radius: (scrollbar_height * 0.5).into(),
            },
            ..renderer::Quad::default()
        },
        thumb_color,
    );
}

fn draw_overflow_button<Renderer: advanced::Renderer>(
    renderer: &mut Renderer,
    bounds: Rectangle,
    bar: &TabBarStyle,
    tab: &crate::style::TabStyle,
    separator_height: f32,
    hovered: bool,
    pressed: bool,
) {
    let (background, color) = if pressed {
        (tab.pressed_background, tab.pressed_text)
    } else if hovered {
        (tab.hovered_background, tab.hovered_text)
    } else {
        (tab.inactive_background, tab.inactive_text)
    };

    renderer.fill_quad(
        renderer::Quad {
            bounds,
            ..renderer::Quad::default()
        },
        background,
    );

    draw_chevron_down(renderer, bounds, color);

    if let Some(separator_color) = bar.separator {
        if separator_height > 0.0 && separator_color.a > 0.0 {
            renderer.fill_quad(
                renderer::Quad {
                    bounds: Rectangle {
                        x: bounds.x,
                        y: bounds.y + bounds.height - separator_height,
                        width: bounds.width,
                        height: separator_height,
                    },
                    ..renderer::Quad::default()
                },
                separator_color,
            );
        }
    }
}

/// Draws a downward-pointing chevron/triangle using only `fill_quad`, so it
/// renders identically on every platform without depending on any font glyph.
fn draw_chevron_down<Renderer: advanced::Renderer>(
    renderer: &mut Renderer,
    bounds: Rectangle,
    color: Color,
) {
    let center_x = bounds.center_x();
    let center_y = bounds.center_y();
    let half_w: f32 = 4.5;
    let half_h: f32 = 3.0;
    let rows: u16 = 6;
    let row_height = (half_h * 2.0) / f32::from(rows);

    for i in 0..rows {
        let offset = f32::from(i) * row_height;
        let progress = offset / (half_h * 2.0);
        let row_width = half_w * 2.0 * (1.0 - progress);
        let row_x = center_x - row_width * 0.5;
        let row_y = center_y - half_h + offset;
        let height = row_height.min(center_y + half_h - row_y);

        renderer.fill_quad(
            renderer::Quad {
                bounds: Rectangle {
                    x: row_x,
                    y: row_y,
                    width: row_width,
                    height,
                },
                ..renderer::Quad::default()
            },
            color,
        );
    }
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
            let movement = if shift {
                Vector::new(y, x)
            } else {
                Vector::new(x, y)
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
pub(crate) fn is_dragging<Theme: menu::Catalog + 'static>(tab_strip_tree: Option<&Tree>) -> bool {
    tab_strip_tree.is_some_and(|tree| {
        let state = tree.state.downcast_ref::<TabStripState<Theme>>();
        state.dragging || state.drag_pending
    })
}

/// Whether a tab label drag has passed the threshold (show grab cursor, etc.).
pub(crate) fn is_tab_drag_active<Theme: menu::Catalog + 'static>(
    tab_strip_tree: Option<&Tree>,
) -> bool {
    tab_strip_tree.is_some_and(|tree| tree.state.downcast_ref::<TabStripState<Theme>>().dragging)
}

fn set_scrollbar_fade_duration<Theme: menu::Catalog>(
    state: &mut TabStripState<Theme>,
    duration: Duration,
) {
    if state.scrollbar_fade_duration == duration {
        return;
    }

    state.scrollbar_visibility = state.scrollbar_visibility.clone().duration(duration);
    state.scrollbar_fade_duration = duration;
}

fn set_scrollbar_animation_enabled<Theme: menu::Catalog>(
    state: &mut TabStripState<Theme>,
    animated: bool,
) {
    state.scrollbar_animated = animated;
}

fn set_scrollbar_visibility<Theme: menu::Catalog>(
    state: &mut TabStripState<Theme>,
    visible: bool,
    at: Instant,
) -> bool {
    let changed = state.scrollbar_visibility.value() != visible;

    if visible {
        state.scrollbar_visibility = Animation::new(true).duration(state.scrollbar_fade_duration);
    } else if state.scrollbar_animated {
        state.scrollbar_visibility.go_mut(false, at);
    } else {
        state.scrollbar_visibility = Animation::new(false).duration(state.scrollbar_fade_duration);
    }

    changed
}

fn scrollbar_alpha<Theme: menu::Catalog>(state: &TabStripState<Theme>, at: Instant) -> f32 {
    if state.scrollbar_drag.is_some() {
        1.0
    } else {
        state
            .scrollbar_visibility
            .interpolate(0.0_f32, 1.0_f32, at)
            .clamp(0.0, 1.0)
    }
}

fn scrollbar_is_interactive<Theme: menu::Catalog>(
    state: &TabStripState<Theme>,
    at: Instant,
) -> bool {
    state.scrollbar_drag.is_some() || scrollbar_alpha(state, at) > 0.0
}

fn tab_row_cursor(tab_bounds: Rectangle, cursor: Cursor, scroll_offset: f32) -> Cursor {
    if cursor_over_tab_bar(tab_bounds, cursor) {
        cursor + Vector::new(scroll_offset, 0.0)
    } else {
        Cursor::Unavailable
    }
}

/// Sync hover / hide state from a parent widget (e.g. [`TabDock`](crate::widget::tab_dock::TabDock)).
pub(crate) fn sync_hover_in_tree<Message, Theme: menu::Catalog + 'static>(
    tab_strip_tree: &mut Tree,
    tab_bounds: Rectangle,
    cursor: Cursor,
    show_scrollbar: bool,
    shell: &mut Shell<'_, Message>,
) {
    let state = tab_strip_tree.state.downcast_mut::<TabStripState<Theme>>();
    sync_tab_bar_hover(state, tab_bounds, cursor, show_scrollbar, shell);
}

fn sync_tab_bar_hover<Message, Theme: menu::Catalog>(
    state: &mut TabStripState<Theme>,
    tab_bounds: Rectangle,
    cursor: Cursor,
    show_scrollbar: bool,
    shell: &mut Shell<'_, Message>,
) {
    if !show_scrollbar {
        return;
    }

    let over = cursor_over_tab_bar(tab_bounds, cursor);
    let now = Instant::now();
    let mut changed = false;

    if state.tab_bar_hovered != over {
        state.tab_bar_hovered = over;
        changed = true;
    }

    if set_scrollbar_visibility(state, over, now) {
        changed = true;
    }

    if changed {
        shell.request_redraw();
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for TabStrip<'_, Message, Theme, Renderer>
where
    Message: Clone + 'static,
    Theme: Catalog
        + button::Catalog
        + container::Catalog
        + menu::Catalog
        + text::Catalog
        + Clone
        + PartialEq
        + 'static,
    Renderer: advanced::Renderer + advanced::text::Renderer + 'static,
    <Theme as button::Catalog>::Class<'static>: From<button::StyleFn<'static, Theme>>,
    <Theme as container::Catalog>::Class<'static>: From<container::StyleFn<'static, Theme>>,
    for<'b> <Theme as text::Catalog>::Class<'b>: From<text::StyleFn<'b, Theme>>,
{
    fn tag(&self) -> Tag {
        Tag::of::<TabStripState<Theme>>()
    }

    fn state(&self) -> State {
        State::new(TabStripState::new(
            self.resolved_theme(),
            self.scrollbar_fade_duration,
            self.scrollbar_animated,
        ))
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
        let bar_height = self.tab_bar_height;
        let max = limits.max();
        let viewport_width = max.width;
        let viewport_limits = layout::Limits::new(Size::ZERO, Size::new(f32::INFINITY, bar_height));
        let row_node = compose::child_layout(
            &mut self.tabs_row,
            &mut tree.children[0],
            renderer,
            &viewport_limits,
        );
        let content_width = row_node.size().width;

        let state = tree.state.downcast_mut::<TabStripState<Theme>>();
        let show_overflow_button = content_width > viewport_width + f32::EPSILON;
        let row_viewport_width = if show_overflow_button {
            (viewport_width - OVERFLOW_BUTTON_TOTAL_WIDTH).max(0.0)
        } else {
            viewport_width
        };
        state.content_width = content_width;
        state.viewport_width = row_viewport_width;
        state.show_overflow_button = show_overflow_button;
        if !show_overflow_button {
            state.overflow_ui.borrow_mut().open = false;
            state.overflow_button_hovered = false;
            state.overflow_menu_hovered = None;
            state.overflow_options.clear();
        }
        let max_offset = max_scroll_offset(content_width, row_viewport_width);
        state.scroll_offset = clamp_scroll_offset(state.scroll_offset, max_offset);
        if let Some(target) = state.overflow_ui.borrow_mut().reveal_tab.take() {
            if let Some(index) = self.tabs.iter().position(|tab| tab.id == target) {
                if let Some(tab_node) = row_node.children().get(index) {
                    state.scroll_offset = ensure_tab_visible(
                        tab_node.bounds(),
                        0.0,
                        state.scroll_offset,
                        row_viewport_width,
                        max_offset,
                    );
                }
            }
        }

        layout::Node::with_children(Size::new(viewport_width, bar_height), vec![row_node])
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
        let row_bounds = visible_row_bounds(tab_bounds, state.viewport_width);
        let visible_bounds = row_bounds.intersection(viewport).unwrap_or(row_bounds);
        let overflow = state.show_overflow_button;
        let overflow_pressed = state.overflow_ui.borrow().pressed;

        renderer.fill_quad(
            renderer::Quad {
                bounds: tab_bounds,
                ..renderer::Quad::default()
            },
            bar.background,
        );

        if let Some(separator_color) = bar.separator {
            let sep_h = self.separator_height;
            if sep_h > 0.0 && separator_color.a > 0.0 {
                let bounds = Rectangle {
                    x: tab_bounds.x,
                    y: tab_bounds.y + tab_bounds.height - sep_h,
                    width: tab_bounds.width,
                    height: sep_h,
                };
                renderer.fill_quad(
                    renderer::Quad {
                        bounds,
                        ..renderer::Quad::default()
                    },
                    separator_color,
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
                            height: (tab_bounds.height - btn_bounds.y + tab_bounds.y + 1.0)
                                .max(0.0),
                        };
                        renderer.fill_quad(
                            renderer::Quad {
                                bounds: fill,
                                ..renderer::Quad::default()
                            },
                            dock_style.tab.active_background,
                        );
                        let accent_h = self.tab_accent_height.max(0.0);
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
                let content_cursor = tab_row_cursor(row_bounds, cursor, translation.x);
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
                        self.insert_marker_width,
                    ) {
                        renderer.fill_quad(
                            renderer::Quad {
                                bounds: marker,
                                ..renderer::Quad::default()
                            },
                            insert_marker_color(drop_overlay, state.drag_blocked),
                        );
                    }
                }
            });

            if self.show_scrollbar && overflow {
                let alpha = scrollbar_alpha(state, Instant::now());
                if let Some(metrics) = scrollbar_metrics(
                    row_bounds,
                    self.scrollbar_height,
                    self.scrollbar_attachment,
                    self.scrollbar_thumb_min_width,
                    scroll_offset,
                    state.content_width,
                    state.viewport_width,
                ) {
                    let thumb_hovered =
                        cursor.position().is_some_and(|p| metrics.thumb.contains(p));
                    draw_scrollbar(
                        &metrics,
                        thumb_hovered || state.scrollbar_drag.is_some(),
                        bar,
                        self.scrollbar_height,
                        alpha,
                        renderer,
                    );
                }
            }
        });

        if overflow {
            let button_bounds = overflow_button_bounds(tab_bounds, state.viewport_width);
            draw_overflow_button(
                renderer,
                button_bounds,
                bar,
                tab_style,
                self.separator_height,
                state.overflow_button_hovered,
                overflow_pressed,
            );
        }
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
        let threshold = self.drag_threshold;
        let cursor_pos = cursor.position();
        let now = Instant::now();
        let mut row_refresh: Option<(Option<NodeId>, Option<NodeId>)> = None;

        {
            let state = tree.state.downcast_mut::<TabStripState<Theme>>();
            set_scrollbar_fade_duration(state, self.scrollbar_fade_duration);
            set_scrollbar_animation_enabled(state, self.scrollbar_animated);
            let row_bounds = visible_row_bounds(tab_bounds, state.viewport_width);
            let overflow_button_bounds = overflow_button_bounds(tab_bounds, state.viewport_width);
            let over_row = cursor
                .position()
                .is_some_and(|point| row_bounds.contains(point));
            let over_overflow_button = state.show_overflow_button
                && cursor
                    .position()
                    .is_some_and(|point| overflow_button_bounds.contains(point));
            let max_offset = max_scroll_offset(state.content_width, state.viewport_width);

            if state.built_theme != current_theme {
                state.built_theme.clone_from(&current_theme);
                row_refresh = Some((state.hovered_tab, visual_pressed_tab(state)));
            }

            if state.suppress_hover && state.hovered_tab.is_some() {
                state.hovered_tab = None;
                row_refresh = Some((None, visual_pressed_tab(state)));
                shell.request_redraw();
            }

            let hovered = if state.suppress_hover {
                None
            } else if over_row {
                cursor_pos.and_then(|pos| {
                    let row_layout = layout.children().next()?;
                    hit_test_tab(&row_layout, &self.tabs, state.scroll_offset, pos)
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
                    if state.scrollbar_visibility.is_animating(*now) {
                        Shell::replace_redraw_request(shell, window::RedrawRequest::NextFrame);
                    }
                }
            }

            if let Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) = event {
                state.keyboard_modifiers = *modifiers;
            }

            let scrollbar_metrics = scrollbar_metrics(
                row_bounds,
                self.scrollbar_height,
                self.scrollbar_attachment,
                self.scrollbar_thumb_min_width,
                state.scroll_offset,
                state.content_width,
                state.viewport_width,
            );
            let scrollbar_interactive = self.show_scrollbar && scrollbar_is_interactive(state, now);

            let mut captured_scrollbar = false;
            let mut captured_wheel = false;
            let mut captured_label = false;

            match event {
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                    if over_overflow_button {
                        let mut overflow_ui = state.overflow_ui.borrow_mut();
                        overflow_ui.open = !overflow_ui.open;
                        overflow_ui.pressed = true;
                        if !overflow_ui.open {
                            state.overflow_menu_hovered = None;
                        }
                        shell.capture_event();
                        shell.request_redraw();
                        captured_label = true;
                    }
                    if !captured_label && scrollbar_interactive {
                        if let (Some(pos), Some(metrics)) = (cursor_pos, scrollbar_metrics.as_ref())
                        {
                            if metrics.thumb.contains(pos) {
                                state.scrollbar_drag = Some(ScrollbarDrag {
                                    grab_x: pos.x - metrics.thumb.x,
                                });
                                let _ = set_scrollbar_visibility(state, true, now);
                                shell.capture_event();
                                shell.request_redraw();
                                captured_scrollbar = true;
                            } else if metrics.track.contains(pos) {
                                let click =
                                    (pos.x - metrics.track.x - metrics.thumb.width * 0.5).max(0.0);
                                let travel = (metrics.track.width - metrics.thumb.width).max(0.0);
                                if travel > 0.0 {
                                    state.scroll_offset = clamp_scroll_offset(
                                        (click / travel) * metrics.max_offset,
                                        metrics.max_offset,
                                    );
                                }
                                let _ = set_scrollbar_visibility(state, true, now);
                                shell.capture_event();
                                shell.request_redraw();
                                captured_scrollbar = true;
                            }
                        }
                    }
                    if !captured_scrollbar && !captured_label {
                        if let (Some(pos), Some(row_layout)) =
                            (cursor_pos, layout.children().next())
                        {
                            let on_close = hit_test_close_button(
                                &row_layout,
                                &self.tabs,
                                state.scroll_offset,
                                pos,
                                self.close_button_size,
                                self.close_button_margin_right,
                            );
                            if !on_close {
                                if let Some(tab_id) =
                                    hit_test_tab(&row_layout, &self.tabs, state.scroll_offset, pos)
                                {
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
                    if over_overflow_button != state.overflow_button_hovered {
                        state.overflow_button_hovered = over_overflow_button;
                        shell.request_redraw();
                    }
                    if self.show_scrollbar {
                        if let (Some(pos), Some(drag), Some(metrics)) =
                            (cursor_pos, state.scrollbar_drag, scrollbar_metrics.as_ref())
                        {
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
                    if state.overflow_ui.borrow().pressed {
                        state.overflow_ui.borrow_mut().pressed = false;
                        shell.request_redraw();
                    }
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
                Event::Mouse(mouse::Event::WheelScrolled { delta })
                    if over_row && max_offset > 0.0 =>
                {
                    let shift = state.keyboard_modifiers.shift();
                    let dx = scroll_delta_x(*delta, shift);
                    if dx.abs() > f32::EPSILON {
                        state.scroll_offset =
                            clamp_scroll_offset(state.scroll_offset + dx, max_offset);
                        if self.show_scrollbar {
                            let _ = set_scrollbar_visibility(state, true, now);
                        }
                        shell.capture_event();
                        shell.request_redraw();
                        captured_wheel = true;
                    }
                }
                Event::Keyboard(keyboard::Event::KeyPressed { key, .. })
                    if matches!(key, keyboard::Key::Named(keyboard::key::Named::Escape))
                        && state.overflow_ui.borrow().open =>
                {
                    let mut overflow_ui = state.overflow_ui.borrow_mut();
                    overflow_ui.open = false;
                    overflow_ui.pressed = false;
                    state.overflow_menu_hovered = None;
                    shell.capture_event();
                    shell.request_redraw();
                }
                _ => {}
            }

            if !over_overflow_button && state.overflow_button_hovered {
                state.overflow_button_hovered = false;
                shell.request_redraw();
            }

            if self.show_scrollbar {
                let hovered = if scrollbar_interactive {
                    scrollbar_metrics
                        .as_ref()
                        .is_some_and(|metrics| cursor_pos.is_some_and(|p| metrics.thumb.contains(p)))
                } else {
                    false
                };

                if hovered != state.scrollbar_thumb_hovered {
                    state.scrollbar_thumb_hovered = hovered;
                    shell.request_redraw();
                }
            }

            let forward_wheel = !matches!(event, Event::Mouse(mouse::Event::WheelScrolled { .. }));

            if !captured_scrollbar && !captured_label && (forward_wheel || !captured_wheel) {
                if let Some(row_layout) = layout.children().next() {
                    let content_cursor = if state.suppress_hover {
                        Cursor::Unavailable
                    } else {
                        tab_row_cursor(row_bounds, cursor, state.scroll_offset)
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
                row_bounds,
                cursor,
                self.show_scrollbar,
                shell,
            );
        };

        if let Some((hovered, pressed)) = row_refresh {
            if let Some(t) = &current_theme {
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
        let row_bounds = visible_row_bounds(tab_bounds, state.viewport_width);
        if state.show_overflow_button
            && cursor.position().is_some_and(|point| {
                overflow_button_bounds(tab_bounds, state.viewport_width).contains(point)
            })
        {
            return mouse::Interaction::Pointer;
        }
        if self.show_scrollbar && scrollbar_is_interactive(state, Instant::now()) {
            if let Some(metrics) = scrollbar_metrics(
                row_bounds,
                self.scrollbar_height,
                self.scrollbar_attachment,
                self.scrollbar_thumb_min_width,
                state.scroll_offset,
                state.content_width,
                state.viewport_width,
            ) {
                if cursor
                    .position()
                    .is_some_and(|p| metrics.thumb.contains(p) || metrics.track.contains(p))
                {
                    return mouse::Interaction::Pointer;
                }
            }
        }

        let content_cursor = tab_row_cursor(row_bounds, cursor, state.scroll_offset);
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

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        _renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let state = tree.state.downcast_mut::<TabStripState<Theme>>();
        if !state.overflow_ui.borrow().open {
            return None;
        }

        let row_layout = layout.children().next()?;
        state.overflow_options = hidden_tabs(
            &row_layout,
            &self.tabs,
            layout.bounds().x,
            state.scroll_offset,
            state.viewport_width,
        );
        if state.overflow_options.is_empty() {
            state.overflow_ui.borrow_mut().open = false;
            return None;
        }

        let button_bounds = overflow_button_bounds(layout.bounds(), state.viewport_width);
        let on_event = Rc::clone(&self.on_event);
        let pane_id = self.pane_id;
        let overflow_ui = Rc::clone(&state.overflow_ui);

        let menu = menu::Menu::new(
            &mut state.overflow_menu_state,
            &state.overflow_options,
            &mut state.overflow_menu_hovered,
            move |option: HiddenTabOption| {
                let mut overflow_ui = overflow_ui.borrow_mut();
                overflow_ui.open = false;
                overflow_ui.pressed = false;
                overflow_ui.reveal_tab = Some(option.id);
                (on_event)(DockAction::Tab(TabAction::Select {
                    pane: pane_id,
                    panel: option.id,
                }))
            },
            None,
            &state.overflow_menu_class,
        )
        .width(button_bounds.width.max(160.0))
        .padding(OVERFLOW_MENU_PADDING)
        .text_size(iced::Pixels(self.tab_text_size))
        .overlay(
            layout.position() + translation + Vector::new(state.viewport_width, 0.0),
            *viewport,
            button_bounds.height,
            Length::Shrink,
        );

        Some(overlay::Element::new(Box::new(OverflowMenuOverlay {
            menu,
            button_bounds,
            ui: Rc::clone(&state.overflow_ui),
        })))
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
        + button::Catalog
        + container::Catalog
        + menu::Catalog
        + text::Catalog
        + Clone
        + PartialEq
        + 'static,
    Renderer: advanced::Renderer + advanced::text::Renderer + 'static,
    <Theme as button::Catalog>::Class<'static>: From<button::StyleFn<'static, Theme>>,
    <Theme as container::Catalog>::Class<'static>: From<container::StyleFn<'static, Theme>>,
    for<'b> <Theme as text::Catalog>::Class<'b>: From<text::StyleFn<'b, Theme>>,
{
    fn from(widget: TabStrip<'a, Message, Theme, Renderer>) -> Self {
        Element::new(widget)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ensure_tab_visible, scrollbar_alpha, scrollbar_is_interactive, set_scrollbar_visibility,
        tab_fully_visible, ScrollbarDrag, TabStripState,
    };
    use iced::time::{Duration, Instant};
    use iced::{Rectangle, Theme};

    #[test]
    fn tab_visibility_requires_full_containment() {
        let row_x = 0.0;
        let viewport_width = 100.0;

        assert!(tab_fully_visible(
            Rectangle::new((10.0, 0.0).into(), (40.0, 20.0).into()),
            row_x,
            0.0,
            viewport_width,
        ));
        assert!(!tab_fully_visible(
            Rectangle::new((-1.0, 0.0).into(), (40.0, 20.0).into()),
            row_x,
            0.0,
            viewport_width,
        ));
        assert!(!tab_fully_visible(
            Rectangle::new((80.0, 0.0).into(), (30.0, 20.0).into()),
            row_x,
            0.0,
            viewport_width,
        ));
    }

    #[test]
    fn ensure_tab_visible_reveals_left_and_right_clipped_tabs() {
        let row_x = 0.0;
        let viewport_width = 100.0;
        let max_offset = 200.0;

        assert_eq!(
            ensure_tab_visible(
                Rectangle::new((40.0, 0.0).into(), (30.0, 20.0).into()),
                row_x,
                0.0,
                viewport_width,
                max_offset,
            ),
            0.0
        );
        assert_eq!(
            ensure_tab_visible(
                Rectangle::new((10.0, 0.0).into(), (30.0, 20.0).into()),
                row_x,
                40.0,
                viewport_width,
                max_offset,
            ),
            10.0
        );
        assert_eq!(
            ensure_tab_visible(
                Rectangle::new((120.0, 0.0).into(), (40.0, 20.0).into()),
                row_x,
                0.0,
                viewport_width,
                max_offset,
            ),
            60.0
        );
    }

    #[test]
    fn entering_tab_bar_shows_scrollbar_immediately() {
        let mut state = TabStripState::<Theme>::new(None, Duration::from_millis(500), true);
        let start = Instant::now();

        assert!(set_scrollbar_visibility(&mut state, true, start));
        assert!(state.scrollbar_visibility.value());
        assert_eq!(scrollbar_alpha(&state, start), 1.0);
        assert_eq!(scrollbar_alpha(&state, start + Duration::from_millis(250)), 1.0);
    }

    #[test]
    fn leaving_tab_bar_starts_immediate_fade_out_and_disables_hit_testing_after_completion() {
        let mut state = TabStripState::<Theme>::new(None, Duration::from_millis(500), true);
        let start = Instant::now();
        let visible_at = start + Duration::from_millis(500);

        let _ = set_scrollbar_visibility(&mut state, true, start);
        let _ = set_scrollbar_visibility(&mut state, false, visible_at);

        assert!(!state.scrollbar_visibility.value());

        let mid = visible_at + Duration::from_millis(250);
        let alpha = scrollbar_alpha(&state, mid);
        assert!(alpha > 0.0 && alpha < 1.0);

        let end = visible_at + Duration::from_millis(500);
        assert_eq!(scrollbar_alpha(&state, end), 0.0);
        assert!(!scrollbar_is_interactive(&state, end));
    }

    #[test]
    fn reentering_during_fade_reverses_toward_visible() {
        let mut state = TabStripState::<Theme>::new(None, Duration::from_millis(500), true);
        let start = Instant::now();
        let visible_at = start + Duration::from_millis(500);
        let fade_mid = visible_at + Duration::from_millis(250);

        let _ = set_scrollbar_visibility(&mut state, true, start);
        let _ = set_scrollbar_visibility(&mut state, false, visible_at);
        let fading_alpha = scrollbar_alpha(&state, fade_mid);

        let _ = set_scrollbar_visibility(&mut state, true, fade_mid);
        let reversed_alpha = scrollbar_alpha(&state, fade_mid);

        assert!(reversed_alpha > fading_alpha);
        assert_eq!(reversed_alpha, 1.0);
    }

    #[test]
    fn dragging_keeps_scrollbar_fully_visible() {
        let mut state = TabStripState::<Theme>::new(None, Duration::from_millis(500), true);
        let start = Instant::now();
        let visible_at = start + Duration::from_millis(500);
        let hidden_at = visible_at + Duration::from_millis(500);

        let _ = set_scrollbar_visibility(&mut state, true, start);
        let _ = set_scrollbar_visibility(&mut state, false, visible_at);
        state.scrollbar_drag = Some(ScrollbarDrag { grab_x: 4.0 });

        assert_eq!(scrollbar_alpha(&state, hidden_at), 1.0);
        assert!(scrollbar_is_interactive(&state, hidden_at));
    }

    #[test]
    fn disabling_animation_hides_scrollbar_immediately() {
        let mut state = TabStripState::<Theme>::new(None, Duration::from_millis(500), false);
        let start = Instant::now();
        let visible_at = start + Duration::from_millis(500);

        let _ = set_scrollbar_visibility(&mut state, true, start);
        let _ = set_scrollbar_visibility(&mut state, false, visible_at);

        assert_eq!(scrollbar_alpha(&state, visible_at), 0.0);
        assert!(!scrollbar_is_interactive(&state, visible_at));
    }
}
