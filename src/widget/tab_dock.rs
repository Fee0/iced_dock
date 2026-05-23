use std::cell::RefCell;
use std::rc::Rc;

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::tree::{State, Tag, Tree};
use iced::advanced::widget::{Operation, Widget};
use iced::advanced::{Clipboard, Renderer as AdvRenderer, Shell};
use iced::mouse::{self, Cursor};
use iced::widget::{button, container, mouse_area, row, text, Space};
use iced::{Color, Element, Event, Length, Padding, Rectangle, Size, Theme};

use crate::manager::DockManager;
use crate::model::NodeId;
use crate::style::{close_button_style, tab_button_style, DockStyle};
use crate::widget::compose;
use crate::widget::dock::{handle_dock_message, DockWidgetState};
use crate::widget::message::{DockMessage, TabMessage};

fn layout_theme() -> Theme {
    Theme::Dark
}

fn drop_zone_rect(bounds: Rectangle, zone: crate::manager::DropZone, edge: f32) -> Rectangle {
    let w = bounds.width;
    let h = bounds.height;
    match zone {
        crate::manager::DropZone::Left => Rectangle {
            width: w * edge,
            ..bounds
        },
        crate::manager::DropZone::Right => Rectangle {
            x: bounds.x + w * (1.0 - edge),
            width: w * edge,
            ..bounds
        },
        crate::manager::DropZone::Top => Rectangle {
            height: h * edge,
            ..bounds
        },
        crate::manager::DropZone::Bottom => Rectangle {
            y: bounds.y + h * (1.0 - edge),
            height: h * edge,
            ..bounds
        },
        crate::manager::DropZone::Center => Rectangle {
            x: bounds.x + w * edge,
            y: bounds.y + h * edge,
            width: w * (1.0 - 2.0 * edge),
            height: h * (1.0 - 2.0 * edge),
        },
    }
}

fn pane_inset(style: &DockStyle) -> f32 {
    style.window.padding + style.window.border.width
}

fn has_tab_strip(tabs: &[TabInfo]) -> bool {
    tabs.len() > 1
}

fn tab_child_index(tabs: &[TabInfo]) -> Option<usize> {
    has_tab_strip(tabs).then_some(2)
}

fn active_tab_index(tabs: &[TabInfo], active_tab: NodeId) -> Option<usize> {
    tabs.iter().position(|tab| tab.id == active_tab)
}

fn draw_tab_strip_background(
    tab_layout: &Layout<'_>,
    active_tab_index: Option<usize>,
    tab_bar_background: Color,
    active_background: Color,
    renderer: &mut iced::Renderer,
) {
    let tab_bounds = tab_layout.bounds();
    renderer.fill_quad(
        renderer::Quad {
            bounds: tab_bounds,
            ..renderer::Quad::default()
        },
        tab_bar_background,
    );

    let Some(active_i) = active_tab_index else {
        return;
    };
    let Some(row_layout) = tab_layout.children().next() else {
        return;
    };
    let Some(active_layout) = row_layout.children().nth(active_i) else {
        return;
    };

    let btn_bounds = active_layout.bounds();
    // Extend 1px upward to cover any subpixel gap with the content above.
    let fill = Rectangle {
        x: btn_bounds.x,
        y: tab_bounds.y - 1.0,
        width: btn_bounds.width,
        height: tab_bounds.height + 1.0,
    };
    renderer.fill_quad(
        renderer::Quad {
            bounds: fill,
            ..renderer::Quad::default()
        },
        active_background,
    );
}

#[derive(Debug, Clone)]
pub struct TabInfo {
    pub id: NodeId,
    pub title: String,
}

#[derive(Default)]
struct TabDockState {
    drag_pending: bool,
    drag_start: Option<iced::Point>,
    dragging: bool,
    hover_zone: Option<crate::manager::DropZone>,
}

pub struct TabDock<'a, Message> {
    dock_state: Rc<RefCell<DockWidgetState>>,
    pub pane_id: NodeId,
    pub can_drag: bool,
    pub tabs: Vec<TabInfo>,
    pub active_tab: NodeId,
    pub chrome: Element<'a, Message, Theme, iced::Renderer>,
    pub tab_strip: Option<Element<'a, Message, Theme, iced::Renderer>>,
    pub content: Element<'a, Message, Theme, iced::Renderer>,
    on_event: Rc<dyn Fn(DockMessage) -> Message>,
    style: Rc<dyn Fn(&Theme) -> DockStyle>,
}

impl<'a, Message: Clone + 'static> TabDock<'a, Message> {
    pub fn new(
        dock_state: Rc<RefCell<DockWidgetState>>,
        pane_id: NodeId,
        title: String,
        can_close: bool,
        can_drag: bool,
        tabs: Vec<TabInfo>,
        active_tab: NodeId,
        content: Element<'a, Message, Theme, iced::Renderer>,
        on_event: Rc<dyn Fn(DockMessage) -> Message>,
        style: Rc<dyn Fn(&Theme) -> DockStyle>,
    ) -> Self {
        let layout_style = (style)(&layout_theme());
        let chrome = build_chrome::<Message>(
            &layout_style,
            title,
            can_close,
            can_drag,
            active_tab,
            on_event.clone(),
        );
        let tab_strip = build_tab_strip::<Message>(
            &layout_style,
            pane_id,
            tabs.clone(),
            active_tab,
            on_event.clone(),
        );
        Self {
            dock_state,
            pane_id,
            can_drag,
            tabs,
            active_tab,
            chrome,
            tab_strip,
            content,
            on_event,
            style,
        }
    }

    fn layout_style(&self) -> DockStyle {
        (self.style)(&layout_theme())
    }

    fn is_dragging(&self, tree: &Tree) -> bool {
        self.dock_state.borrow().drag.is_some()
            || tree.state.downcast_ref::<TabDockState>().dragging
    }

    fn register_drop_target(&self, bounds: Rectangle) {
        self.dock_state
            .borrow_mut()
            .drop_targets
            .push((self.pane_id, bounds));
    }
}

fn build_chrome<Message: Clone + 'static>(
    style: &DockStyle,
    title: String,
    can_close: bool,
    can_drag: bool,
    active_tab: NodeId,
    on_event: Rc<dyn Fn(DockMessage) -> Message>,
) -> Element<'static, Message, Theme, iced::Renderer> {
    let tb = &style.title_bar;
    let title_label = text(title)
        .size(tb.text_size)
        .color(tb.text_color);
    let mut drag_strip = mouse_area(
        Space::new()
            .width(Length::Fill)
            .height(Length::Fill),
    );
    if can_drag {
        drag_strip = drag_strip.interaction(mouse::Interaction::Grab);
    }
    let drag_strip = container(drag_strip)
        .width(Length::Fill)
        .height(Length::Fill);

    let close: Element<'_, Message, Theme, iced::Renderer> = if can_close {
        let on_event = on_event.clone();
        let cb = &tb.close_button;
        button(text("×").size(cb.text_size))
            .padding(cb.padding)
            .style(close_button_style(cb))
            .on_press_with(move || {
                (on_event)(DockMessage::Tab(TabMessage::Close { panel: active_tab }))
            })
            .into()
    } else {
        Space::new()
            .width(Length::Fixed(tb.close_button_width))
            .into()
    };

    row![title_label, drag_strip, close]
        .height(Length::Fixed(tb.height))
        .align_y(iced::Alignment::Center)
        .padding(Padding {
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
            left: 10.0,
        })
        .into()
}

fn build_tab_strip<Message: Clone + 'static>(
    style: &DockStyle,
    pane_id: NodeId,
    tabs: Vec<TabInfo>,
    active_tab: NodeId,
    on_event: Rc<dyn Fn(DockMessage) -> Message>,
) -> Option<Element<'static, Message, Theme, iced::Renderer>> {
    if tabs.len() <= 1 {
        return None;
    }
    let mut style = style.clone();
    style.sync_tab_appearance();
    let bar = &style.tab_bar;
    let tab_style = &style.tab;
    let mut strip = row![]
        .spacing(bar.spacing)
        .padding(bar.padding)
        .width(Length::Fill)
        .height(Length::Fixed(bar.height))
        .align_y(iced::Alignment::Start);
    for tab in tabs {
        let on_event = on_event.clone();
        let tab_id = tab.id;
        let is_active = tab.id == active_tab;
        let label = container(
            text(tab.title.clone()).size(tab_style.text_size),
        )
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
    Some(
        container(strip)
            .width(Length::Fill)
            .height(Length::Fixed(bar.height))
            .into(),
    )
}

impl<'a, Message> Widget<Message, Theme, iced::Renderer> for TabDock<'a, Message>
where
    Message: Clone + 'static,
{
    fn tag(&self) -> Tag {
        Tag::of::<TabDockState>()
    }

    fn state(&self) -> State {
        State::new(TabDockState::default())
    }

    fn children(&self) -> Vec<Tree> {
        let mut trees = vec![Tree::new(&self.chrome), Tree::new(&self.content)];
        if let Some(tabs) = &self.tab_strip {
            trees.push(Tree::new(tabs));
        }
        trees
    }

    fn diff(&self, tree: &mut Tree) {
        if tree.children.is_empty() {
            tree.children.push(Tree::new(&self.chrome));
            tree.children.push(Tree::new(&self.content));
            if let Some(tabs) = &self.tab_strip {
                tree.children.push(Tree::new(tabs));
            }
            return;
        }
        tree.children[0].diff(&self.chrome);
        tree.children[1].diff(&self.content);
        if let Some(tabs) = &self.tab_strip {
            if tree.children.len() < 3 {
                tree.children.push(Tree::new(tabs));
            } else {
                tree.children[2].diff(tabs);
            }
        } else if tree.children.len() > 2 {
            tree.children.pop();
        }
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
        let style = self.layout_style();
        let max = limits.max();
        let inset = pane_inset(&style);
        let inner_w = (max.width - 2.0 * inset).max(0.0);
        let inner_h = (max.height - 2.0 * inset).max(0.0);
        let title_h = style.title_bar.height;
        let tab_h = if self.tabs.len() > 1 {
            style.tab_bar.height
        } else {
            0.0
        };
        let content_h = (inner_h - title_h - tab_h).max(0.0);
        let mut y = inset;

        let chrome_limits = layout::Limits::new(Size::ZERO, Size::new(inner_w, title_h));
        let mut chrome_node =
            compose::child_layout(&mut self.chrome, &mut tree.children[0], renderer, &chrome_limits);
        chrome_node.move_to_mut((inset, y));
        y += title_h;

        let content_limits = layout::Limits::new(Size::ZERO, Size::new(inner_w, content_h));
        let mut content_node = compose::child_layout(
            &mut self.content,
            &mut tree.children[1],
            renderer,
            &content_limits,
        );
        content_node.move_to_mut((inset, y));
        y += content_h;

        let mut nodes = vec![chrome_node, content_node];

        if let (Some(tabs), Some(tab_idx)) = (&mut self.tab_strip, tab_child_index(&self.tabs)) {
            let tab_limits = layout::Limits::new(Size::ZERO, Size::new(inner_w, tab_h));
            let mut tab_node = compose::child_layout(
                tabs,
                &mut tree.children[tab_idx],
                renderer,
                &tab_limits,
            );
            tab_node.move_to_mut((inset, y));
            nodes.push(tab_node);
        }

        layout::Node::with_children(Size::new(max.width, max.height), nodes)
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
        let mut dock_style = (self.style)(theme);
        dock_style.sync_tab_appearance();

        let pane_bounds = layout.bounds();
        let window = &dock_style.window;

        renderer.fill_quad(
            renderer::Quad {
                bounds: pane_bounds,
                border: window.border,
                ..renderer::Quad::default()
            },
            window.background,
        );

        if let Some(chrome_layout) = layout.children().next() {
            let chrome_bounds = chrome_layout.bounds();
            renderer.fill_quad(
                renderer::Quad {
                    bounds: chrome_bounds,
                    ..renderer::Quad::default()
                },
                dock_style.title_bar.background,
            );
            let sep = Rectangle {
                x: chrome_bounds.x,
                y: chrome_bounds.y + chrome_bounds.height - 1.0,
                width: chrome_bounds.width,
                height: 1.0,
            };
            renderer.fill_quad(
                renderer::Quad {
                    bounds: sep,
                    ..renderer::Quad::default()
                },
                Color::from_rgba(0.0, 0.0, 0.0, 0.35),
            );
        }

        if let Some(chrome_layout) = layout.children().next() {
            compose::child_draw(
                &self.chrome,
                &tree.children[0],
                renderer,
                theme,
                style,
                chrome_layout,
                cursor,
                viewport,
            );
        }
        if let Some(content_layout) = layout.children().nth(1) {
            compose::child_draw(
                &self.content,
                &tree.children[1],
                renderer,
                theme,
                style,
                content_layout,
                cursor,
                viewport,
            );
        }
        if let Some(tab_idx) = tab_child_index(&self.tabs) {
            if let Some(tab_layout) = layout.children().nth(tab_idx) {
                draw_tab_strip_background(
                    &tab_layout,
                    active_tab_index(&self.tabs, self.active_tab),
                    dock_style.tab_bar.background,
                    dock_style.tab.active_background,
                    renderer,
                );
            }
        }
        if let (Some(tab_idx), Some(tabs)) = (tab_child_index(&self.tabs), &self.tab_strip) {
            if let (Some(tab_layout), Some(child_tree)) =
                (layout.children().nth(tab_idx), tree.children.get(tab_idx))
            {
                compose::child_draw(
                    tabs,
                    child_tree,
                    renderer,
                    theme,
                    style,
                    tab_layout,
                    cursor,
                    viewport,
                );
            }
        }

        let drag_session = self.dock_state.borrow().drag;
        let show_overlay = self.is_dragging(tree);

        if show_overlay {
            if let Some(content_layout) = layout.children().nth(1) {
                let bounds = content_layout.bounds();
                let zone = drag_session
                    .filter(|s| s.hover_target == Some(self.pane_id))
                    .and_then(|s| s.operation)
                    .and_then(|_| cursor.position())
                    .and_then(|point| DockManager::hit_test_drop_zone(bounds, point))
                    .or_else(|| {
                        cursor
                            .position_over(bounds)
                            .and_then(|point| DockManager::hit_test_drop_zone(bounds, point))
                    });

                if let Some(zone) = zone {
                    let highlight = dock_style.drop_overlay.color;
                    let edge = dock_style.drop_overlay.edge_fraction;
                    let zone_bounds = drop_zone_rect(bounds, zone, edge);
                    renderer.fill_quad(
                        renderer::Quad {
                            bounds: zone_bounds,
                            ..Default::default()
                        },
                        highlight,
                    );
                }
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
        let layout_style = self.layout_style();
        let dragging = self.is_dragging(tree);
        let state = tree.state.downcast_mut::<TabDockState>();

        if let Some(content_layout) = layout.children().nth(1) {
            self.register_drop_target(content_layout.bounds());
        }

        if let Some(chrome_layout) = layout.children().next() {
            compose::child_update(
                &mut self.chrome,
                &mut tree.children[0],
                event,
                chrome_layout,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
        }
        if let Some(content_layout) = layout.children().nth(1) {
            compose::child_update(
                &mut self.content,
                &mut tree.children[1],
                event,
                content_layout,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
        }
        if let (Some(tab_idx), Some(tabs)) = (tab_child_index(&self.tabs), &mut self.tab_strip) {
            if let (Some(tab_layout), Some(child_tree)) =
                (layout.children().nth(tab_idx), tree.children.get_mut(tab_idx))
            {
                compose::child_update(
                    tabs,
                    child_tree,
                    event,
                    tab_layout,
                    cursor,
                    renderer,
                    clipboard,
                    shell,
                    viewport,
                );
            }
        }

        if let Some(bar_layout) = layout.children().next() {
            let drag_bounds = bar_layout
                .children()
                .nth(1)
                .map(|drag_layout| drag_layout.bounds())
                .unwrap_or_else(|| bar_layout.bounds());
            let threshold = layout_style.title_bar.drag_threshold;
            match event {
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                    if self.can_drag {
                        if let Some(pos) = cursor.position() {
                            if drag_bounds.contains(pos) {
                                state.drag_pending = true;
                                state.drag_start = Some(pos);
                            }
                        }
                    }
                }
                Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                    if state.drag_pending {
                        if let (Some(start), Some(pos)) = (state.drag_start, cursor.position()) {
                            let dx = pos.x - start.x;
                            let dy = pos.y - start.y;
                            if (dx * dx + dy * dy).sqrt() >= threshold {
                                state.dragging = true;
                                state.drag_pending = false;
                                shell.publish((self.on_event)(DockMessage::Tab(
                                    TabMessage::DragStarted {
                                        source_pane: self.pane_id,
                                        source_panel: self.active_tab,
                                    },
                                )));
                                shell.request_redraw();
                            }
                        }
                    }
                }
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                    state.drag_pending = false;
                    state.drag_start = None;
                    let was_dragging = state.dragging;
                    state.dragging = false;

                    if was_dragging || self.dock_state.borrow().drag.is_some() {
                        if let Some(pos) = cursor.position() {
                            shell.publish((self.on_event)(DockMessage::Tab(
                                TabMessage::DragEnded { cursor: pos },
                            )));
                        } else {
                            let _ = handle_dock_message(
                                &mut self.dock_state.borrow_mut(),
                                DockMessage::Tab(TabMessage::DragCancelled),
                            );
                        }
                        shell.invalidate_layout();
                        shell.invalidate_widgets();
                        shell.request_redraw();
                    }
                }
                _ => {}
            }
        }

        if dragging {
            if let Event::Mouse(mouse::Event::CursorMoved { .. }) = event {
                if let Some(pos) = cursor.position() {
                    if let Some(content_layout) = layout.children().nth(1) {
                        let bounds = content_layout.bounds();
                        state.hover_zone = cursor
                            .position_over(bounds)
                            .and_then(|p| DockManager::hit_test_drop_zone(bounds, p));
                    }
                    shell.publish((self.on_event)(DockMessage::Tab(TabMessage::DragMoved {
                        cursor: pos,
                    })));
                    shell.request_redraw();
                }
            }
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
        if self.is_dragging(tree) {
            return mouse::Interaction::Grab;
        }

        let mut interaction = mouse::Interaction::None;
        if let Some(chrome_layout) = layout.children().next() {
            interaction = interaction.max(compose::child_mouse_interaction(
                &self.chrome,
                &tree.children[0],
                chrome_layout,
                cursor,
                viewport,
                renderer,
            ));
        }
        if let Some(content_layout) = layout.children().nth(1) {
            interaction = interaction.max(compose::child_mouse_interaction(
                &self.content,
                &tree.children[1],
                content_layout,
                cursor,
                viewport,
                renderer,
            ));
        }
        interaction
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        if let Some(chrome_layout) = layout.children().next() {
            compose::child_operate(
                &mut self.chrome,
                &mut tree.children[0],
                chrome_layout,
                renderer,
                operation,
            );
        }
        if let Some(content_layout) = layout.children().nth(1) {
            compose::child_operate(
                &mut self.content,
                &mut tree.children[1],
                content_layout,
                renderer,
                operation,
            );
        }
    }
}

impl<'a, Message> From<TabDock<'a, Message>> for Element<'a, Message, Theme, iced::Renderer>
where
    Message: Clone + 'static,
{
    fn from(widget: TabDock<'a, Message>) -> Self {
        Element::new(widget)
    }
}
