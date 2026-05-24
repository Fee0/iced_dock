use std::rc::Rc;

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::tree::{State, Tag, Tree};
use iced::advanced::widget::{Operation, Widget};
use iced::advanced::{Clipboard, Renderer as AdvRenderer, Shell};
use iced::keyboard;
use iced::mouse::{self, Cursor};
use iced::widget::{button, container, row, text};
use iced::window;
use iced::{
    Element, Event, Length, Padding, Rectangle, Size, Theme, Vector,
};

use crate::model::NodeId;
use crate::style::{tab_button_style, DockStyle};
use crate::widget::compose;
use crate::widget::message::{DockMessage, TabMessage};
use crate::widget::tab_dock::TabInfo;

#[derive(Default)]
struct TabStripState {
    scroll_offset: f32,
    content_width: f32,
    viewport_width: f32,
    tab_bar_hovered: bool,
    scrollbar_visible: bool,
    hide_at: Option<iced::time::Instant>,
    scrollbar_drag: Option<ScrollbarDrag>,
    scrollbar_thumb_hovered: bool,
    keyboard_modifiers: keyboard::Modifiers,
}

#[derive(Debug, Clone, Copy)]
struct ScrollbarDrag {
    grab_x: f32,
}

pub struct TabStrip<'a, Message> {
    tabs: Vec<TabInfo>,
    active_tab: NodeId,
    tabs_row: Element<'a, Message, Theme, iced::Renderer>,
    style: Rc<dyn Fn(&Theme) -> DockStyle>,
    hide_delay: iced::time::Duration,
}

impl<'a, Message: Clone + 'static> TabStrip<'a, Message> {
    pub fn new(
        pane_id: NodeId,
        tabs: Vec<TabInfo>,
        active_tab: NodeId,
        on_event: Rc<dyn Fn(DockMessage) -> Message>,
        style: Rc<dyn Fn(&Theme) -> DockStyle>,
        hide_delay: iced::time::Duration,
    ) -> Self {
        let layout_style = (style)(&Theme::Dark);
        let tabs_row = build_tabs_row(&layout_style, pane_id, &tabs, active_tab, on_event.clone());
        Self {
            tabs,
            active_tab,
            tabs_row,
            style,
            hide_delay,
        }
    }

    fn layout_style(&self, theme: &Theme) -> DockStyle {
        let mut style = (self.style)(theme);
        style.sync_tab_appearance();
        style
    }

    fn active_tab_index(&self) -> Option<usize> {
        self.tabs.iter().position(|tab| tab.id == self.active_tab)
    }
}

fn build_tabs_row<Message: Clone + 'static>(
    style: &DockStyle,
    pane_id: NodeId,
    tabs: &[TabInfo],
    active_tab: NodeId,
    on_event: Rc<dyn Fn(DockMessage) -> Message>,
) -> Element<'static, Message, Theme, iced::Renderer> {
    let bar = &style.tab_bar;
    let tab_style = &style.tab;
    let mut strip = row![]
        .spacing(bar.spacing)
        .padding(bar.padding)
        .width(Length::Shrink)
        .height(Length::Fixed(bar.height))
        .align_y(iced::Alignment::Start);
    for tab in tabs {
        let on_event = on_event.clone();
        let tab_id = tab.id;
        let is_active = tab.id == active_tab;
        let label = container(text(tab.title.clone()).size(tab_style.text_size))
            .height(Length::Fill)
            .center_y(Length::Fill);
        let btn = button(label)
            .padding(Padding {
                top: 0.0,
                bottom: 0.0,
                left: tab_style.padding[1],
                right: tab_style.padding[1],
            })
            .height(Length::Fixed(bar.height))
            .style(tab_button_style(tab_style, is_active))
            .on_press_with(move || {
                (on_event)(DockMessage::Tab(TabMessage::Select {
                    pane: pane_id,
                    panel: tab_id,
                }))
            });
        strip = strip.push(btn);
    }
    strip.into()
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
    let thumb_width = (track.width * ratio).max(2.0);
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

fn draw_scrollbar(
    metrics: &ScrollbarMetrics,
    thumb_hovered: bool,
    bar: &crate::style::TabBarStyle,
    renderer: &mut iced::Renderer,
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
            -movement.y * 60.0
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

/// Pending scrollbar hide deadline, if a delayed hide was scheduled.
pub(crate) fn pending_hide_deadline(tree: &Tree) -> Option<iced::time::Instant> {
    tree.state
        .downcast_ref::<TabStripState>()
        .hide_at
}

/// Ensure a redraw is scheduled when the hide deadline elapses.
///
/// Does not replace an already-requested [`RedrawRequest::NextFrame`] so an immediate
/// hover-clear redraw can run in the same frame.
pub(crate) fn schedule_hide_redraw<Message>(
    tree: &Tree,
    shell: &mut Shell<'_, Message>,
) {
    let Some(deadline) = pending_hide_deadline(tree) else {
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
pub(crate) fn sync_hover_in_tree<Message>(
    tab_strip_tree: &mut Tree,
    tab_bounds: Rectangle,
    cursor: Cursor,
    hide_delay: iced::time::Duration,
    shell: &mut Shell<'_, Message>,
) {
    let state = tab_strip_tree.state.downcast_mut::<TabStripState>();
    sync_tab_bar_hover(state, tab_bounds, cursor, hide_delay, shell);
}

fn sync_tab_bar_hover<Message>(
    state: &mut TabStripState,
    tab_bounds: Rectangle,
    cursor: Cursor,
    hide_delay: iced::time::Duration,
    shell: &mut Shell<'_, Message>,
) {
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

impl<Message> Widget<Message, Theme, iced::Renderer> for TabStrip<'_, Message>
where
    Message: Clone + 'static,
{
    fn tag(&self) -> Tag {
        Tag::of::<TabStripState>()
    }

    fn state(&self) -> State {
        State::new(TabStripState::default())
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
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let style = self.layout_style(&Theme::Dark);
        let bar_height = style.tab_bar.height;
        let max = limits.max();
        let viewport_width = max.width;
        // Measure the tab row at natural width so we can detect horizontal overflow.
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

        let state = tree.state.downcast_mut::<TabStripState>();
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
        renderer: &mut iced::Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
    ) {
        let dock_style = self.layout_style(theme);
        let bar = &dock_style.tab_bar;
        let state = tree.state.downcast_ref::<TabStripState>();
        let tab_bounds = layout.bounds();
        let Some(row_layout) = layout.children().next() else {
            return;
        };

        let scroll_offset = state.scroll_offset;
        let visible_bounds = tab_bounds.intersection(viewport).unwrap_or(tab_bounds);
        let overflow = state.content_width > state.viewport_width + f32::EPSILON;
        let scrollbar_band = if overflow {
            bar.scrollbar_height
        } else {
            0.0
        };

        renderer.fill_quad(
            renderer::Quad {
                bounds: tab_bounds,
                ..renderer::Quad::default()
            },
            bar.background,
        );

        let translation = Vector::new(scroll_offset, 0.0);

        renderer.with_layer(visible_bounds, |renderer| {
            renderer.with_translation(Vector::new(-translation.x, -translation.y), |renderer| {
                if let Some(active_i) = self.active_tab_index() {
                    if let Some(active_layout) = row_layout.children().nth(active_i) {
                        let btn_bounds = active_layout.bounds();
                        let fill = Rectangle {
                            x: btn_bounds.x,
                            y: tab_bounds.y - 1.0,
                            width: btn_bounds.width,
                            height: (tab_bounds.height - scrollbar_band + 1.0).max(0.0),
                        };
                        renderer.fill_quad(
                            renderer::Quad {
                                bounds: fill,
                                ..renderer::Quad::default()
                            },
                            dock_style.tab.active_background,
                        );
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
            });
        });

        let show_scrollbar = state.scrollbar_visible
            && state
                .hide_at
                .is_none_or(|deadline| iced::time::Instant::now() < deadline);

        if overflow && show_scrollbar {
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
        let state = tree.state.downcast_mut::<TabStripState>();
        let tab_bounds = layout.bounds();
        let max_offset = max_scroll_offset(state.content_width, state.viewport_width);
        let dock_style = self.layout_style(&Theme::Dark);
        let bar = &dock_style.tab_bar;

        let cursor_pos = cursor.position();
        let over_tab_bar = cursor_over_tab_bar(tab_bounds, cursor);

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

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
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
                            state.scroll_offset =
                                clamp_scroll_offset(
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
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
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
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if state.scrollbar_drag.take().is_some() {
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                if over_tab_bar && max_offset > 0.0 {
                    let shift = state.keyboard_modifiers.shift();
                    let dx = scroll_delta_x(*delta, shift);
                    if dx.abs() > f32::EPSILON {
                        state.scroll_offset =
                            clamp_scroll_offset(state.scroll_offset + dx, max_offset);
                        state.scrollbar_visible = true;
                        state.hide_at = None;
                        shell.capture_event();
                        shell.request_redraw();
                        captured_wheel = true;
                    }
                }
            }
            _ => {}
        }

        if let Some(metrics) = scrollbar_metrics.as_ref() {
            let hovered = cursor_pos.is_some_and(|p| metrics.thumb.contains(p));
            if hovered != state.scrollbar_thumb_hovered {
                state.scrollbar_thumb_hovered = hovered;
                shell.request_redraw();
            }
        }

        let forward_wheel = !matches!(
            event,
            Event::Mouse(mouse::Event::WheelScrolled { .. })
        );

        if !captured_scrollbar && (forward_wheel || !captured_wheel) {
            if let Some(row_layout) = layout.children().next() {
                let content_cursor =
                    tab_row_cursor(tab_bounds, cursor, state.scroll_offset);
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

        sync_tab_bar_hover(state, tab_bounds, cursor, self.hide_delay, shell);
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<TabStripState>();
        if state.scrollbar_drag.is_some() {
            return mouse::Interaction::Grabbing;
        }

        let tab_bounds = layout.bounds();
        let dock_style = self.layout_style(&Theme::Dark);
        if let Some(metrics) = scrollbar_metrics(
            tab_bounds,
            &dock_style.tab_bar,
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
        renderer: &iced::Renderer,
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

impl<'a, Message> From<TabStrip<'a, Message>> for Element<'a, Message, Theme, iced::Renderer>
where
    Message: Clone + 'static,
{
    fn from(widget: TabStrip<'a, Message>) -> Self {
        Element::new(widget)
    }
}
