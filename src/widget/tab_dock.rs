use std::rc::Rc;

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::tree::{State, Tag, Tree};
use iced::advanced::widget::{Operation, Widget};
use iced::advanced::{Clipboard, Renderer as AdvRenderer, Shell};
use iced::mouse::{self, Cursor};
use iced::widget::{button, mouse_area, row, text, Space};
use iced::{Border, Element, Event, Length, Rectangle, Size, Theme};

use crate::model::NodeId;
use crate::manager::DockManager;
use crate::widget::compose;
use crate::widget::message::{DockMessage, TabMessage};

pub const TITLE_BAR_HEIGHT: f32 = 28.0;
pub const TAB_STRIP_HEIGHT: f32 = 24.0;
pub const DRAG_THRESHOLD: f32 = 6.0;
pub const CLOSE_BUTTON_WIDTH: f32 = 36.0;

fn drop_zone_rect(bounds: Rectangle, zone: crate::manager::DropZone) -> Rectangle {
    let w = bounds.width;
    let h = bounds.height;
    const EDGE: f32 = 0.2;
    match zone {
        crate::manager::DropZone::Left => Rectangle {
            width: w * EDGE,
            ..bounds
        },
        crate::manager::DropZone::Right => Rectangle {
            x: bounds.x + w * (1.0 - EDGE),
            width: w * EDGE,
            ..bounds
        },
        crate::manager::DropZone::Top => Rectangle {
            height: h * EDGE,
            ..bounds
        },
        crate::manager::DropZone::Bottom => Rectangle {
            y: bounds.y + h * (1.0 - EDGE),
            height: h * EDGE,
            ..bounds
        },
        crate::manager::DropZone::Center => Rectangle {
            x: bounds.x + w * EDGE,
            y: bounds.y + h * EDGE,
            width: w * (1.0 - 2.0 * EDGE),
            height: h * (1.0 - 2.0 * EDGE),
        },
    }
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
    pub group_id: NodeId,
    pub can_drag: bool,
    pub tabs: Vec<TabInfo>,
    pub active_tab: NodeId,
    pub chrome: Element<'a, Message, Theme, iced::Renderer>,
    pub tab_strip: Option<Element<'a, Message, Theme, iced::Renderer>>,
    pub content: Element<'a, Message, Theme, iced::Renderer>,
    pub drag_active: bool,
    on_event: Rc<dyn Fn(DockMessage) -> Message>,
}

impl<'a, Message: Clone + 'static> TabDock<'a, Message> {
    pub fn new(
        group_id: NodeId,
        title: String,
        can_close: bool,
        can_drag: bool,
        tabs: Vec<TabInfo>,
        active_tab: NodeId,
        content: Element<'a, Message, Theme, iced::Renderer>,
        drag_active: bool,
        on_event: Rc<dyn Fn(DockMessage) -> Message>,
    ) -> Self {
        let chrome = build_chrome::<Message>(title, can_close, can_drag, active_tab, on_event.clone());
        let tab_strip =
            build_tab_strip::<Message>(group_id, tabs.clone(), active_tab, on_event.clone());
        Self {
            group_id,
            can_drag,
            tabs,
            active_tab,
            chrome,
            tab_strip,
            content,
            drag_active,
            on_event,
        }
    }
}

fn build_chrome<Message: Clone + 'static>(
    title: String,
    can_close: bool,
    can_drag: bool,
    active_tab: NodeId,
    on_event: Rc<dyn Fn(DockMessage) -> Message>,
) -> Element<'static, Message, Theme, iced::Renderer> {
    let title_label = text(title).size(13);
    let drag_strip = mouse_area(Space::new().width(Length::Fill).height(Length::Fill));
    let drag_strip = if can_drag {
        drag_strip.interaction(mouse::Interaction::Grab)
    } else {
        drag_strip
    };

    let close: Element<'_, Message, Theme, iced::Renderer> = if can_close {
        let on_event = on_event.clone();
        button(text("×").size(14))
            .padding([2, 8])
            .on_press_with(move || {
                (on_event)(DockMessage::Tab(TabMessage::Close { tab: active_tab }))
            })
            .into()
    } else {
        Space::new().width(Length::Fixed(28.0)).into()
    };

    row![title_label.width(Length::FillPortion(1)), drag_strip, close]
        .height(Length::Fixed(TITLE_BAR_HEIGHT))
        .align_y(iced::Alignment::Center)
        .into()
}

fn build_tab_strip<Message: Clone + 'static>(
    group_id: NodeId,
    tabs: Vec<TabInfo>,
    active_tab: NodeId,
    on_event: Rc<dyn Fn(DockMessage) -> Message>,
) -> Option<Element<'static, Message, Theme, iced::Renderer>> {
    if tabs.len() <= 1 {
        return None;
    }
    let mut strip = row![].spacing(4).padding([2, 4]);
    for tab in tabs {
        let label = text(tab.title.clone()).size(12);
        let on_event = on_event.clone();
        let tab_id = tab.id;
        let is_active = tab.id == active_tab;
        let btn = if is_active {
            button(label)
                .padding([2, 8])
                .style(button::primary)
        } else {
            button(label).padding([2, 8])
        };
        let btn = btn.on_press_with(move || {
            (on_event)(DockMessage::Tab(TabMessage::Select {
                group: group_id,
                tab: tab_id,
            }))
        });
        strip = strip.push(btn);
    }
    Some(
        strip
            .height(Length::Fixed(TAB_STRIP_HEIGHT))
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
        let max = limits.max();
        let tab_h = if self.tabs.len() > 1 {
            TAB_STRIP_HEIGHT
        } else {
            0.0
        };
        let content_h = (max.height - TITLE_BAR_HEIGHT - tab_h).max(0.0);

        let chrome_limits = layout::Limits::new(
            Size::ZERO,
            Size::new(max.width, TITLE_BAR_HEIGHT),
        );
        let mut chrome_node =
            compose::child_layout(&mut self.chrome, &mut tree.children[0], renderer, &chrome_limits);
        chrome_node.move_to_mut((0.0, 0.0));

        let content_limits = layout::Limits::new(
            Size::ZERO,
            Size::new(max.width, content_h),
        );
        let mut content_node = compose::child_layout(
            &mut self.content,
            &mut tree.children[1],
            renderer,
            &content_limits,
        );
        content_node.move_to_mut((0.0, TITLE_BAR_HEIGHT));

        let mut nodes = vec![chrome_node, content_node];
        if let (Some(tabs), Some(tab_tree)) = (&mut self.tab_strip, tree.children.get_mut(2)) {
            let tab_limits = layout::Limits::new(
                Size::ZERO,
                Size::new(max.width, TAB_STRIP_HEIGHT),
            );
            let mut tab_node = compose::child_layout(tabs, tab_tree, renderer, &tab_limits);
            tab_node.move_to_mut((0.0, TITLE_BAR_HEIGHT + content_h));
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
        let palette = theme.extended_palette();
        let pane_bounds = layout.bounds();

        renderer.fill_quad(
            renderer::Quad {
                bounds: pane_bounds,
                border: Border {
                    width: 1.0,
                    color: palette.background.weak.color,
                    radius: 0.0.into(),
                },
                ..renderer::Quad::default()
            },
            palette.background.base.color,
        );

        if let Some(chrome_layout) = layout.children().next() {
            let chrome_bounds = chrome_layout.bounds();
            renderer.fill_quad(
                renderer::Quad {
                    bounds: chrome_bounds,
                    ..renderer::Quad::default()
                },
                palette.background.strong.color,
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
        if let (Some(tab_layout), Some(tabs)) = (layout.children().nth(2), &self.tab_strip) {
            if let Some(child_tree) = tree.children.get(2) {
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

        if self.drag_active || tree.state.downcast_ref::<TabDockState>().dragging {
            if let Some(content_layout) = layout.children().nth(1) {
                let bounds = content_layout.bounds();
                if let Some(point) = cursor.position_over(bounds) {
                    if let Some(zone) = DockManager::hit_test_drop_zone(bounds, point) {
                        let palette = theme.extended_palette();
                        let highlight = iced::Color {
                            a: 0.35,
                            ..palette.primary.base.color
                        };
                        let zone_bounds = drop_zone_rect(bounds, zone);
                        renderer.fill_quad(
                            iced::advanced::renderer::Quad {
                                bounds: zone_bounds,
                                ..Default::default()
                            },
                            highlight,
                        );
                    }
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
        let state = tree.state.downcast_mut::<TabDockState>();

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
        if let (Some(tab_layout), Some(tabs)) = (layout.children().nth(2), &mut self.tab_strip) {
            if let Some(child_tree) = tree.children.get_mut(2) {
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

        // Drag threshold on title bar region (middle third via layout)
        if let Some(bar_layout) = layout.children().next() {
            let bar_bounds = bar_layout.bounds();
            let drag_bounds = Rectangle {
                x: bar_bounds.x,
                y: bar_bounds.y,
                width: (bar_bounds.width - CLOSE_BUTTON_WIDTH).max(0.0),
                height: bar_bounds.height,
            };
            match event {
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                    if self.can_drag {
                        if let Some(pos) = cursor.position() {
                            if drag_bounds.contains(pos) {
                                state.drag_pending = true;
                                state.drag_start = Some(pos);
                                shell.capture_event();
                            }
                        }
                    }
                }
                Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                    if state.drag_pending {
                        if let (Some(start), Some(pos)) =
                            (state.drag_start, cursor.position())
                        {
                            let dx = pos.x - start.x;
                            let dy = pos.y - start.y;
                            if (dx * dx + dy * dy).sqrt() >= DRAG_THRESHOLD {
                                state.dragging = true;
                                state.drag_pending = false;
                                shell.publish((self.on_event)(DockMessage::Tab(
                                    TabMessage::DragStarted {
                                        source_group: self.group_id,
                                        source_tab: self.active_tab,
                                    },
                                )));
                                shell.capture_event();
                            }
                        }
                    }
                }
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                    state.drag_pending = false;
                    state.drag_start = None;
                    state.dragging = false;
                }
                _ => {}
            }
        }

        if state.dragging || self.drag_active {
            if let Event::Mouse(mouse::Event::CursorMoved { .. }) = event {
                if let Some(content_layout) = layout.children().nth(1) {
                    let bounds = content_layout.bounds();
                    if let Some(point) = cursor.position_over(bounds) {
                        state.hover_zone = DockManager::hit_test_drop_zone(bounds, point);
                        if let Some(zone) = state.hover_zone {
                            shell.publish((self.on_event)(DockMessage::Tab(TabMessage::DragMoved {
                                target: self.group_id,
                                zone,
                            })));
                        }
                    } else {
                        state.hover_zone = None;
                    }
                    shell.capture_event();
                }
            }
            if let Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) = event {
                let drop_zone = layout.children().nth(1).and_then(|content_layout| {
                    let bounds = content_layout.bounds();
                    cursor
                        .position_over(bounds)
                        .and_then(|point| DockManager::hit_test_drop_zone(bounds, point))
                });
                state.hover_zone = None;
                state.dragging = false;

                if self.drag_active {
                    if let Some(zone) = drop_zone {
                        shell.publish((self.on_event)(DockMessage::Tab(TabMessage::DragEnded {
                            target: self.group_id,
                            zone,
                        })));
                    }
                    shell.capture_event();
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
