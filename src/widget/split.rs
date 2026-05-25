use std::cell::RefCell;
use std::rc::Rc;

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::tree::{State, Tag, Tree};
use iced::advanced::widget::{Operation, Widget};
use iced::advanced::{Clipboard, Shell};
use iced::mouse::{self, Cursor};
use iced::{Border, Element, Event, Length, Rectangle, Size};

use crate::model::{Axis, NodeId};
use crate::style::{Catalog, DockStyle};
use crate::widget::action::DockAction;
use crate::widget::compose;

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

pub struct SplitContainer<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Theme: Catalog,
    Renderer: iced::advanced::Renderer,
{
    pub group_id: NodeId,
    pub axis: Axis,
    pub proportions: Vec<f32>,
    pub children: Vec<Element<'a, Message, Theme, Renderer>>,
    on_event: Rc<dyn Fn(DockAction) -> Message>,
    class: Rc<<Theme as Catalog>::Class<'static>>,
    theme: Rc<RefCell<Option<Theme>>>,
    min_pane_width: f32,
    min_pane_height: f32,
}

impl<'a, Message, Theme, Renderer> SplitContainer<'a, Message, Theme, Renderer>
where
    Theme: Catalog + Clone,
    Renderer: iced::advanced::Renderer,
{
    pub fn new(
        group_id: NodeId,
        axis: Axis,
        proportions: Vec<f32>,
        children: Vec<Element<'a, Message, Theme, Renderer>>,
        on_event: Rc<dyn Fn(DockAction) -> Message>,
        class: Rc<<Theme as Catalog>::Class<'static>>,
        theme: Rc<RefCell<Option<Theme>>>,
        min_pane_width: f32,
        min_pane_height: f32,
    ) -> Self {
        Self {
            group_id,
            axis,
            proportions,
            children,
            on_event,
            class,
            theme,
            min_pane_width,
            min_pane_height,
        }
    }

    fn layout_style_resolved(&self) -> DockStyle {
        match *self.theme.borrow() {
            Some(ref t) => Catalog::style(t, &self.class),
            None => crate::style::default(&iced::Theme::Dark),
        }
    }

    fn layout_style(&self, theme: &Theme) -> DockStyle {
        Catalog::style(theme, &self.class)
    }
}

fn normalize_to_sum(sizes: &mut [f32], target: f32) {
    let sum: f32 = sizes.iter().sum();
    if sizes.is_empty() {
        return;
    }
    if sum <= 1e-6 {
        let each = target / sizes.len() as f32;
        sizes.fill(each);
        return;
    }
    let scale = target / sum;
    for s in sizes.iter_mut() {
        *s *= scale;
    }
}

/// Shrink panes that are above `min_pane_size` until the group fits in `available`.
/// Never reduces a pane below `min_pane_size`.
fn shrink_to_fit(sizes: &mut [f32], min_pane_size: f32, available: f32) {
    let mut used: f32 = sizes.iter().sum();
    if used <= available + 1e-4 {
        return;
    }

    for _ in 0..sizes.len() {
        let excess = used - available;
        if excess <= 1e-4 {
            return;
        }
        let flexible_sum: f32 = sizes.iter().map(|&s| (s - min_pane_size).max(0.0)).sum();
        if flexible_sum <= 1e-4 {
            return;
        }
        for s in sizes.iter_mut() {
            let flex = (*s - min_pane_size).max(0.0);
            if flex > 0.0 {
                *s = (*s - excess * (flex / flexible_sum)).max(min_pane_size);
            }
        }
        used = sizes.iter().sum();
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

    let mut sizes: Vec<f32> = props.iter().map(|p| p * available).collect();
    let min_total = min_pane_size * count as f32;

    if min_total <= available {
        for size in &mut sizes {
            *size = size.max(min_pane_size);
        }
        shrink_to_fit(&mut sizes, min_pane_size, available);
    }

    normalize_to_sum(&mut sizes, available);
    sizes
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for SplitContainer<'_, Message, Theme, Renderer>
where
    Message: Clone + 'static,
    Theme: Catalog + Clone + 'static,
    Renderer: iced::advanced::Renderer,
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
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let dock_style = self.layout_style_resolved();
        let splitter_size = dock_style.splitter.size;
        let splitter_gap = dock_style.splitter.gap;
        let is_horizontal = self.axis == Axis::Horizontal;
        let min_pane_main = if is_horizontal {
            self.min_pane_width
        } else {
            self.min_pane_height
        };

        let state = tree.state.downcast_mut::<SplitWidgetState>();
        let size = limits.max();
        let count = self.children.len();
        if count == 0 {
            state.splitter_bounds.clear();
            return layout::Node::new(Size::ZERO);
        }

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
            min_pane_main,
        );

        let mut children_nodes = Vec::with_capacity(count);
        state.splitter_bounds.clear();

        let mut offset = 0.0_f32;
        for (i, child) in self.children.iter_mut().enumerate() {
            let pane_main = pane_sizes.get(i).copied().unwrap_or(0.0);
            let child_limits = if is_horizontal {
                layout::Limits::new(
                    Size::new(pane_main, size.height),
                    Size::new(pane_main, size.height),
                )
            } else {
                layout::Limits::new(
                    Size::new(size.width, pane_main),
                    Size::new(size.width, pane_main),
                )
            };
            let child_tree = &mut tree.children[i];
            let mut node = child
                .as_widget_mut()
                .layout(child_tree, renderer, &child_limits);
            if is_horizontal {
                node.move_to_mut((offset, 0.0));
                offset += pane_main;
            } else {
                node.move_to_mut((0.0, offset));
                offset += pane_main;
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
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<SplitWidgetState>();
        let dock_style = self.layout_style(theme);
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
            let hovered = cursor_pos.is_some_and(|p| abs.contains(p));
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
        renderer: &Renderer,
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
        let min_pane_main = if is_horizontal {
            self.min_pane_width
        } else {
            self.min_pane_height
        };

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
                        let pair_total = state.drag_start_left_size + state.drag_start_right_size;
                        let min_left = min_pane_main.min(pair_total);
                        let max_left = (pair_total - min_pane_main).max(min_left);
                        let new_left =
                            (state.drag_start_left_size + delta).clamp(min_left, max_left);
                        let pair_ratio = if pair_total > 0.0 {
                            new_left / pair_total
                        } else {
                            0.5
                        };
                        shell.publish((self.on_event)(DockAction::SplitDrag {
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
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if state.drag_splitter.is_some() =>
            {
                state.drag_splitter = None;
                state.hovered_splitter = cursor
                    .position()
                    .and_then(|p| splitter_under_cursor(p, &state.splitter_bounds, offset));
                shell.request_redraw();
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
        renderer: &Renderer,
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
        renderer: &Renderer,
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

impl<'a, Message, Theme, Renderer> From<SplitContainer<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: Clone + 'static,
    Theme: Catalog + Clone + 'static,
    Renderer: iced::advanced::Renderer + 'static,
{
    fn from(widget: SplitContainer<'a, Message, Theme, Renderer>) -> Self {
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
        let total = 1000.0;
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
    fn compute_pane_sizes_respects_min_pane_size() {
        let total = 500.0;
        let count = 4;
        let splitter_size = 0.5;
        let splitter_gap = 10.0;
        let min_pane_size = 80.0;
        let available = total - (splitter_size + splitter_gap) * 3.0;

        let sizes = compute_pane_sizes(
            total,
            &[0.05, 0.05, 0.45, 0.45],
            count,
            splitter_size,
            splitter_gap,
            min_pane_size,
        );

        for &size in &sizes {
            assert!(
                size >= min_pane_size - 1e-3,
                "pane size {size} below minimum {min_pane_size}"
            );
        }
        let sum: f32 = sizes.iter().sum();
        approx_eq(sum, available);
    }

    #[test]
    fn compute_pane_sizes_fits_all_panes_when_below_min_total() {
        let total = 200.0;
        let splitter_size = 0.5;
        let splitter_gap = 10.0;
        let min_pane_size = 80.0;
        let available = total - (splitter_size + splitter_gap) * 3.0;

        let sizes = compute_pane_sizes(
            total,
            &[0.25, 0.25, 0.25, 0.25],
            4,
            splitter_size,
            splitter_gap,
            min_pane_size,
        );
        assert_eq!(sizes.len(), 4);
        approx_eq(sizes.iter().sum(), available);
        for &size in &sizes {
            assert!(size > 0.0, "each pane must remain visible");
            assert!(size < min_pane_size, "below min when container is tight");
        }
    }

    #[test]
    fn compute_pane_sizes_sum_matches_available() {
        let total = 350.0;
        let available = total - (0.5 + 10.0) * 2.0;
        let sizes = compute_pane_sizes(total, &[0.2, 0.3, 0.5], 3, 0.5, 10.0, 80.0);
        approx_eq(sizes.iter().sum(), available);
    }
}
