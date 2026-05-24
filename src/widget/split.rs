use std::rc::Rc;

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::tree::{State, Tag, Tree};
use iced::advanced::widget::{Operation, Widget};
use iced::advanced::{Clipboard, Renderer as AdvRenderer, Shell};
use iced::mouse::{self, Cursor};
use iced::{Border, Element, Event, Length, Rectangle, Size, Theme};

use crate::model::{Axis, NodeId};
use crate::style::DockStyle;
use crate::widget::compose;
use crate::widget::message::DockMessage;

fn layout_theme() -> Theme {
    Theme::Dark
}

#[derive(Default)]
struct SplitWidgetState {
    splitter_bounds: Vec<Rectangle>,
    drag_splitter: Option<usize>,
    hovered_splitter: Option<usize>,
    drag_start_cursor: f32,
    drag_start_left_size: f32,
    drag_start_right_size: f32,
}

fn splitter_under_cursor(
    cursor_pos: iced::Point,
    bounds: &[Rectangle],
    offset: iced::Vector,
) -> Option<usize> {
    bounds.iter().enumerate().find_map(|(idx, bounds)| {
        let abs = *bounds + offset;
        abs.contains(cursor_pos).then_some(idx)
    })
}

fn pane_main_size(child_layout: &Layout<'_>, is_horizontal: bool) -> f32 {
    if is_horizontal {
        child_layout.bounds().width
    } else {
        child_layout.bounds().height
    }
}

pub struct SplitContainer<'a, Message> {
    pub group_id: NodeId,
    pub axis: Axis,
    pub proportions: Vec<f32>,
    pub children: Vec<Element<'a, Message, Theme, iced::Renderer>>,
    on_event: Rc<dyn Fn(DockMessage) -> Message>,
    style: Rc<dyn Fn(&Theme) -> DockStyle>,
}

impl<'a, Message> SplitContainer<'a, Message> {
    pub fn new(
        group_id: NodeId,
        axis: Axis,
        proportions: Vec<f32>,
        children: Vec<Element<'a, Message, Theme, iced::Renderer>>,
        on_event: Rc<dyn Fn(DockMessage) -> Message>,
        style: Rc<dyn Fn(&Theme) -> DockStyle>,
    ) -> Self {
        Self {
            group_id,
            axis,
            proportions,
            children,
            on_event,
            style,
        }
    }

    fn layout_style(&self) -> DockStyle {
        (self.style)(&layout_theme())
    }
}

fn compute_pane_sizes(
    total: f32,
    proportions: &[f32],
    count: usize,
    splitter_size: f32,
    splitter_gap: f32,
    min_pane_size: f32,
) -> Vec<f32> {
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

    let between = count.saturating_sub(1) as f32;
    let splitter_total = (splitter_size + splitter_gap) * between;
    let available = (total - splitter_total).max(0.0);
    if available <= 0.0 {
        return vec![0.0; count];
    }

    let min_total = min_pane_size * count as f32;
    if min_total > available {
        let scale = available / min_total;
        return vec![min_pane_size * scale; count];
    }

    props.iter().map(|p| p * available).collect()
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
        let dock_style = self.layout_style();
        let splitter_size = dock_style.splitter.size;
        let splitter_gap = dock_style.splitter.gap;
        let min_pane_size = dock_style.splitter.min_pane_size;

        let state = tree.state.downcast_mut::<SplitWidgetState>();
        let size = limits.max();
        let count = self.children.len();
        if count == 0 {
            state.splitter_bounds.clear();
            return layout::Node::new(Size::ZERO);
        }

        let is_horizontal = self.axis == Axis::Horizontal;
        let main_size = if is_horizontal {
            size.width
        } else {
            size.height
        };
        let pane_sizes = compute_pane_sizes(
            main_size,
            &self.proportions,
            count,
            splitter_size,
            splitter_gap,
            min_pane_size,
        );

        let mut children_nodes = Vec::with_capacity(count);
        state.splitter_bounds.clear();

        let mut offset = 0.0f32;
        for (i, child) in self.children.iter_mut().enumerate() {
            let pane_main = pane_sizes.get(i).copied().unwrap_or(min_pane_size);
            let limit_min = min_pane_size.min(pane_main);
            let limit_max = pane_main.max(min_pane_size);
            let child_limits = if is_horizontal {
                layout::Limits::new(
                    Size::new(limit_min, size.height),
                    Size::new(limit_max, size.height),
                )
            } else {
                layout::Limits::new(
                    Size::new(size.width, limit_min),
                    Size::new(size.width, limit_max),
                )
            };
            let child_tree = &mut tree.children[i];
            let mut node = child
                .as_widget_mut()
                .layout(child_tree, renderer, &child_limits);
            if is_horizontal {
                node.move_to_mut((offset, 0.0));
                offset += node.size().width;
            } else {
                node.move_to_mut((0.0, offset));
                offset += node.size().height;
            }
            children_nodes.push(node);

            if i + 1 < count {
                let hit_w = if is_horizontal {
                    splitter_size + splitter_gap
                } else {
                    size.width
                };
                let hit_h = if is_horizontal {
                    size.height
                } else {
                    splitter_size + splitter_gap
                };
                let splitter = Rectangle {
                    x: if is_horizontal { offset } else { 0.0 },
                    y: if is_horizontal { 0.0 } else { offset },
                    width: hit_w,
                    height: hit_h,
                };
                state.splitter_bounds.push(splitter);
                offset += splitter_size + splitter_gap;
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
        let dock_style = (self.style)(theme);
        let split = &dock_style.splitter;

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

        for (idx, &bounds) in state.splitter_bounds.iter().enumerate() {
            let abs = bounds + offset;
            let hovered = cursor_pos.map(|p| abs.contains(p)).unwrap_or(false);
            let dragging_this = state.drag_splitter == Some(idx);
            let color = if dragging_this {
                split.drag_color
            } else if hovered {
                split.hover_color
            } else {
                split.idle_color
            };
            if color.a <= 0.0 {
                continue;
            }
            let line = if self.axis == Axis::Horizontal {
                let w = split.size.max(1.0);
                Rectangle {
                    x: abs.x + (abs.width - w) * 0.5,
                    y: abs.y,
                    width: w,
                    height: abs.height,
                }
            } else {
                let h = split.size.max(1.0);
                Rectangle {
                    x: abs.x,
                    y: abs.y + (abs.height - h) * 0.5,
                    width: abs.width,
                    height: h,
                }
            };
            renderer.fill_quad(
                renderer::Quad {
                    bounds: line,
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
        let min_pane_size = self.layout_style().splitter.min_pane_size;

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(cursor_pos) = cursor.position() {
                    for (idx, bounds) in state.splitter_bounds.iter().enumerate() {
                        let abs = *bounds + offset;
                        if abs.contains(cursor_pos) {
                            let children: Vec<_> = layout.children().collect();
                            if let (Some(left), Some(right)) =
                                (children.get(idx), children.get(idx + 1))
                            {
                                state.drag_splitter = Some(idx);
                                state.drag_start_cursor = if is_horizontal {
                                    cursor_pos.x
                                } else {
                                    cursor_pos.y
                                };
                                state.drag_start_left_size = pane_main_size(left, is_horizontal);
                                state.drag_start_right_size = pane_main_size(right, is_horizontal);
                                shell.capture_event();
                                shell.request_redraw();
                            }
                            return;
                        }
                    }
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let Some(idx) = state.drag_splitter {
                    if let Some(cursor_pos) = cursor.position() {
                        let cursor_main = if is_horizontal {
                            cursor_pos.x
                        } else {
                            cursor_pos.y
                        };
                        let delta = cursor_main - state.drag_start_cursor;
                        let pair_total =
                            state.drag_start_left_size + state.drag_start_right_size;
                        let new_left = (state.drag_start_left_size + delta)
                            .clamp(min_pane_size, pair_total - min_pane_size);
                        let pair_ratio = if pair_total > 0.0 {
                            new_left / pair_total
                        } else {
                            0.5
                        };
                        shell.publish((self.on_event)(DockMessage::SplitDrag {
                            group: self.group_id,
                            splitter_index: idx,
                            pair_ratio,
                        }));
                        shell.capture_event();
                        shell.request_redraw();
                    }
                } else {
                    let new_hover = cursor
                        .position()
                        .and_then(|p| splitter_under_cursor(p, &state.splitter_bounds, offset));
                    if new_hover != state.hovered_splitter {
                        state.hovered_splitter = new_hover;
                        shell.request_redraw();
                    }
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if state.drag_splitter.is_some() {
                    state.drag_splitter = None;
                    state.hovered_splitter = cursor
                        .position()
                        .and_then(|p| splitter_under_cursor(p, &state.splitter_bounds, offset));
                    shell.request_redraw();
                }
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

impl<'a, Message> From<SplitContainer<'a, Message>> for Element<'a, Message, Theme, iced::Renderer>
where
    Message: Clone + 'static,
{
    fn from(widget: SplitContainer<'a, Message>) -> Self {
        Element::new(widget)
    }
}

#[cfg(test)]
mod tests {
    use super::compute_pane_sizes;

    fn approx_eq(a: f32, b: f32) {
        assert!((a - b).abs() < 1e-3, "expected {b}, got {a}");
    }

    #[test]
    fn compute_pane_sizes_outer_panes_unchanged_when_pair_resizes() {
        let total = 500.0;
        let count = 4;
        let splitter_size = 0.5;
        let splitter_gap = 10.0;
        let min_pane_size = 80.0;

        let baseline = compute_pane_sizes(
            total,
            &[0.1, 0.2, 0.3, 0.4],
            count,
            splitter_size,
            splitter_gap,
            min_pane_size,
        );
        let after_pair_resize = compute_pane_sizes(
            total,
            &[0.1, 0.35, 0.15, 0.4],
            count,
            splitter_size,
            splitter_gap,
            min_pane_size,
        );

        approx_eq(baseline[0], after_pair_resize[0]);
        approx_eq(baseline[3], after_pair_resize[3]);
        assert!(after_pair_resize[1] > baseline[1]);
        assert!(after_pair_resize[2] < baseline[2]);
    }

    #[test]
    fn compute_pane_sizes_uniform_scale_when_container_too_small() {
        let sizes = compute_pane_sizes(200.0, &[0.25, 0.25, 0.25, 0.25], 4, 0.5, 10.0, 80.0);
        assert_eq!(sizes.len(), 4);
        let sum: f32 = sizes.iter().sum();
        approx_eq(sum, 200.0 - (0.5 + 10.0) * 3.0);
        for size in sizes {
            assert!(size < 80.0);
        }
    }
}
