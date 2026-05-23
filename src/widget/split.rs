use std::rc::Rc;

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::tree::{State, Tag, Tree};
use iced::advanced::widget::{Operation, Widget};
use iced::advanced::{Clipboard, Renderer as AdvRenderer, Shell};
use iced::mouse::{self, Cursor};
use iced::{Border, Color, Element, Event, Length, Rectangle, Size, Theme};

use crate::model::{Axis, NodeId};
use crate::widget::compose;
use crate::widget::message::DockMessage;

pub const SPLITTER_SIZE: f32 = 5.0;
pub const MIN_PANE_SIZE: f32 = 80.0;

#[derive(Default)]
struct SplitWidgetState {
    splitter_bounds: Vec<Rectangle>,
    drag_splitter: Option<usize>,
}

pub struct SplitContainer<'a, Message> {
    pub group_id: NodeId,
    pub axis: Axis,
    pub proportions: Vec<f32>,
    pub children: Vec<Element<'a, Message, Theme, iced::Renderer>>,
    on_event: Rc<dyn Fn(DockMessage) -> Message>,
}

impl<'a, Message> SplitContainer<'a, Message> {
    pub fn new(
        group_id: NodeId,
        axis: Axis,
        proportions: Vec<f32>,
        children: Vec<Element<'a, Message, Theme, iced::Renderer>>,
        on_event: Rc<dyn Fn(DockMessage) -> Message>,
    ) -> Self {
        Self {
            group_id,
            axis,
            proportions,
            children,
            on_event,
        }
    }
}

fn compute_pane_sizes(total: f32, proportions: &[f32], count: usize) -> Vec<f32> {
    if count == 0 {
        return Vec::new();
    }

    let mut props: Vec<f32> = if proportions.len() == count {
        proportions.to_vec()
    } else {
        vec![1.0; count]
    };
    let sum: f32 = props.iter().sum();
    if sum > 0.0 {
        for p in &mut props {
            *p /= sum;
        }
    }

    let splitter_total = SPLITTER_SIZE * (count.saturating_sub(1)) as f32;
    let available = (total - splitter_total).max(0.0);
    if available <= 0.0 {
        return vec![0.0; count];
    }

    let mut sizes: Vec<f32> = props.iter().map(|p| p * available).collect();
    let min_total = MIN_PANE_SIZE * count as f32;
    if min_total <= available {
        for size in &mut sizes {
            *size = size.max(MIN_PANE_SIZE);
        }
        let used: f32 = sizes.iter().sum();
        if used > available {
            let scale = available / used;
            for size in &mut sizes {
                *size *= scale;
            }
        }
    } else {
        let scale = available / min_total;
        for size in &mut sizes {
            *size = MIN_PANE_SIZE * scale;
        }
    }
    sizes
}

impl<'a, Message> Widget<Message, Theme, iced::Renderer> for SplitContainer<'a, Message>
where
    Message: Clone + 'static,
{
    fn tag(&self) -> Tag {
        Tag::of::<SplitWidgetState>()
    }

    fn state(&self) -> State {
        State::new(SplitWidgetState::default())
    }

    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);
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
        let state = tree.state.downcast_mut::<SplitWidgetState>();
        let size = limits.max();
        let count = self.children.len();
        if count == 0 {
            state.splitter_bounds.clear();
            return layout::Node::new(Size::ZERO);
        }

        let is_horizontal = self.axis == Axis::Horizontal;
        let main_size = if is_horizontal { size.width } else { size.height };
        let pane_sizes = compute_pane_sizes(main_size, &self.proportions, count);

        let mut children_nodes = Vec::with_capacity(count);
        state.splitter_bounds.clear();

        let mut offset = 0.0f32;
        for (i, child) in self.children.iter_mut().enumerate() {
            let pane_main = pane_sizes.get(i).copied().unwrap_or(MIN_PANE_SIZE);
            let child_limits = if is_horizontal {
                layout::Limits::new(
                    Size::new(MIN_PANE_SIZE, size.height),
                    Size::new(pane_main, size.height),
                )
            } else {
                layout::Limits::new(
                    Size::new(size.width, MIN_PANE_SIZE),
                    Size::new(size.width, pane_main),
                )
            };
            let child_tree = &mut tree.children[i];
            let mut node = child.as_widget_mut().layout(child_tree, renderer, &child_limits);
            if is_horizontal {
                node.move_to_mut((offset, 0.0));
                offset += node.size().width;
            } else {
                node.move_to_mut((0.0, offset));
                offset += node.size().height;
            }
            children_nodes.push(node);

            if i + 1 < count {
                let splitter = Rectangle {
                    x: if is_horizontal { offset } else { 0.0 },
                    y: if is_horizontal { 0.0 } else { offset },
                    width: if is_horizontal {
                        SPLITTER_SIZE
                    } else {
                        size.width
                    },
                    height: if is_horizontal {
                        size.height
                    } else {
                        SPLITTER_SIZE
                    },
                };
                state.splitter_bounds.push(splitter);
                offset += SPLITTER_SIZE;
            }
        }

        layout::Node::with_children(size, children_nodes)
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
        let state = tree.state.downcast_ref::<SplitWidgetState>();
        let palette = theme.extended_palette();
        let default_splitter_color = Color {
            a: 0.6,
            ..palette.background.weak.text
        };
        let hover_splitter_color = Color {
            a: 0.85,
            ..palette.primary.weak.color
        };

        let pos = layout.position();
        let offset = iced::Vector::new(pos.x, pos.y);
        let cursor_pos = cursor.position();

        for (i, child_layout) in layout.children().enumerate() {
            if let Some(child) = self.children.get(i) {
                compose::child_draw(
                    child,
                    &tree.children[i],
                    renderer,
                    theme,
                    style,
                    child_layout,
                    cursor,
                    viewport,
                );
            }
        }

        for &bounds in &state.splitter_bounds {
            let abs = bounds + offset;
            let hovered = cursor_pos
                .map(|p| abs.contains(p))
                .unwrap_or(false);
            let color = if hovered {
                hover_splitter_color
            } else {
                default_splitter_color
            };
            renderer.fill_quad(
                renderer::Quad {
                    bounds: abs,
                    border: Border::default(),
                    ..renderer::Quad::default()
                },
                color,
            );
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
        let state = tree.state.downcast_mut::<SplitWidgetState>();
        let is_horizontal = self.axis == Axis::Horizontal;

        for (i, child_layout) in layout.children().enumerate() {
            if let Some(child) = self.children.get_mut(i) {
                compose::child_update(
                    child,
                    &mut tree.children[i],
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

        let pos = layout.position();
        let offset = iced::Vector::new(pos.x, pos.y);
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(cursor_pos) = cursor.position() {
                    for (idx, bounds) in state.splitter_bounds.iter().enumerate() {
                        let abs = *bounds + offset;
                        if abs.contains(cursor_pos) {
                            state.drag_splitter = Some(idx);
                            shell.capture_event();
                            return;
                        }
                    }
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let Some(idx) = state.drag_splitter {
                    if let Some(cursor_pos) = cursor.position() {
                        let total = if is_horizontal {
                            layout.bounds().width
                        } else {
                            layout.bounds().height
                        };
                        let rel = if is_horizontal {
                            (cursor_pos.x - layout.bounds().x) / total
                        } else {
                            (cursor_pos.y - layout.bounds().y) / total
                        };
                        let ratio = rel.clamp(0.15, 0.85);
                        shell.publish((self.on_event)(DockMessage::SplitDrag {
                            group: self.group_id,
                            splitter_index: idx,
                            ratio,
                        }));
                        shell.capture_event();
                    }
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                state.drag_splitter = None;
            }
            _ => {}
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
        let state = tree.state.downcast_ref::<SplitWidgetState>();
        let pos = layout.position();
        let offset = iced::Vector::new(pos.x, pos.y);
        if let Some(cursor_pos) = cursor.position() {
            for bounds in &state.splitter_bounds {
                let abs = *bounds + offset;
                if abs.contains(cursor_pos) {
                    return match self.axis {
                        Axis::Horizontal => mouse::Interaction::ResizingHorizontally,
                        Axis::Vertical => mouse::Interaction::ResizingVertically,
                    };
                }
            }
        }

        let mut interaction = mouse::Interaction::None;
        for (i, child_layout) in layout.children().enumerate() {
            if let Some(child) = self.children.get(i) {
                interaction = interaction.max(compose::child_mouse_interaction(
                    child,
                    &tree.children[i],
                    child_layout,
                    cursor,
                    viewport,
                    renderer,
                ));
            }
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
        for (i, child_layout) in layout.children().enumerate() {
            if let Some(child) = self.children.get_mut(i) {
                compose::child_operate(
                    child,
                    &mut tree.children[i],
                    child_layout,
                    renderer,
                    operation,
                );
            }
        }
    }
}

impl<'a, Message> From<SplitContainer<'a, Message>>
    for Element<'a, Message, Theme, iced::Renderer>
where
    Message: Clone + 'static,
{
    fn from(widget: SplitContainer<'a, Message>) -> Self {
        Element::new(widget)
    }
}
