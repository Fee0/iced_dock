use std::cell::RefCell;
use std::rc::Rc;

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::tree::{State, Tag, Tree};
use iced::advanced::widget::{Operation, Widget};
use iced::advanced::{self, Clipboard, Shell};
use iced::mouse::{self, Cursor};
use iced::touch;
use iced::time::Duration;
use iced::widget::{button, container, text as iced_text};
use iced::{Element, Event, Length, Rectangle, Size};

use crate::manager::{DockManager, DragSession, DropZone, TabBarTarget};
use crate::model::NodeId;
use crate::style::{self, Catalog, DockStyle};
use crate::widget::action::{DockAction, TabAction};
use crate::widget::compose;
use crate::widget::state::DockWidgetState;
use crate::widget::tab_strip::{self, TabStrip};

fn drop_zone_rect(bounds: Rectangle, zone: DropZone, edge: f32) -> Rectangle {
    let w = bounds.width;
    let h = bounds.height;
    match zone {
        DropZone::Left => Rectangle {
            width: w * edge,
            ..bounds
        },
        DropZone::Right => Rectangle {
            x: bounds.x + w * (1.0 - edge),
            width: w * edge,
            ..bounds
        },
        DropZone::Top => Rectangle {
            height: h * edge,
            ..bounds
        },
        DropZone::Bottom => Rectangle {
            y: bounds.y + h * (1.0 - edge),
            height: h * edge,
            ..bounds
        },
        DropZone::Center => Rectangle {
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

#[derive(Debug, Clone)]
pub struct TabInfo {
    pub id: NodeId,
    pub title: String,
    pub can_close: bool,
    pub can_drag: bool,
}

#[derive(Default)]
struct TabDockState;

fn tab_insert_is_noop(
    session: DragSession,
    pane_id: NodeId,
    index: usize,
    tabs: &[TabInfo],
) -> bool {
    if session.source_pane != pane_id {
        return false;
    }
    let Some(from) = tabs.iter().position(|t| t.id == session.source_panel) else {
        return false;
    };
    from == index || from + 1 == index
}

pub struct TabDock<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Theme: Catalog,
    Renderer: advanced::Renderer,
{
    dock_state: Rc<RefCell<DockWidgetState<Theme>>>,
    pub pane_id: NodeId,
    pub tabs: Vec<TabInfo>,
    pub active_tab: NodeId,
    pub tab_strip: Element<'a, Message, Theme, Renderer>,
    pub content: Element<'a, Message, Theme, Renderer>,
    on_event: Rc<dyn Fn(DockAction) -> Message>,
    class: Rc<<Theme as Catalog>::Class<'static>>,
    theme: Rc<RefCell<Option<Theme>>>,
    drop_edge_fraction: f32,
    tab_bar_scrollbar_hide_delay: Duration,
    tab_bar_show_scrollbar: bool,
}

impl<'a, Message, Theme, Renderer> TabDock<'a, Message, Theme, Renderer>
where
    Message: Clone + 'static,
    Theme: Catalog
        + button::Catalog
        + container::Catalog
        + iced_text::Catalog
        + Clone
        + PartialEq
        + 'static,
    Renderer: advanced::Renderer + advanced::text::Renderer + 'static,
    <Theme as button::Catalog>::Class<'static>:
        From<button::StyleFn<'static, Theme>>,
    <Theme as container::Catalog>::Class<'static>:
        From<container::StyleFn<'static, Theme>>,
    for<'b> <Theme as iced_text::Catalog>::Class<'b>:
        From<iced_text::StyleFn<'b, Theme>>,
{
    pub fn new(
        dock_state: Rc<RefCell<DockWidgetState<Theme>>>,
        pane_id: NodeId,
        tabs: Vec<TabInfo>,
        active_tab: NodeId,
        content: Element<'a, Message, Theme, Renderer>,
        on_event: Rc<dyn Fn(DockAction) -> Message>,
        class: Rc<<Theme as Catalog>::Class<'static>>,
        theme: Rc<RefCell<Option<Theme>>>,
        drag_threshold: f32,
        drop_edge_fraction: f32,
        tab_bar_scrollbar_hide_delay: Duration,
        tab_bar_show_scrollbar: bool,
    ) -> Self {
        let tab_strip = TabStrip::new(
            pane_id,
            tabs.clone(),
            active_tab,
            on_event.clone(),
            Rc::clone(&class),
            Rc::clone(&theme),
            drag_threshold,
            drop_edge_fraction,
            tab_bar_scrollbar_hide_delay,
            tab_bar_show_scrollbar,
        )
        .into();
        Self {
            dock_state,
            pane_id,
            tabs,
            active_tab,
            tab_strip,
            content,
            on_event,
            class,
            theme,
            drop_edge_fraction,
            tab_bar_scrollbar_hide_delay,
            tab_bar_show_scrollbar,
        }
    }
}

impl<Message, Theme, Renderer> TabDock<'_, Message, Theme, Renderer>
where
    Theme: Catalog + Clone + 'static,
    Renderer: advanced::Renderer,
{
    fn resolved_theme(&self) -> Option<Theme> {
        self.theme.borrow().clone()
    }

    fn layout_style(&self, theme: &Theme) -> DockStyle {
        Catalog::style(theme, &self.class)
    }

    fn layout_style_or_default(&self) -> DockStyle {
        match self.resolved_theme() {
            Some(ref t) => Catalog::style(t, &self.class),
            None => style::default(&iced::Theme::Dark),
        }
    }

    fn is_dragging(&self, tree: &Tree) -> bool {
        self.dock_state.borrow().drag.is_some()
            || tab_strip::is_dragging::<Theme>(tree.children.first())
    }

    fn register_drop_target(&self, bounds: Rectangle) {
        self.dock_state
            .borrow_mut()
            .drop_targets
            .push((self.pane_id, bounds));
    }

    fn register_tab_bar_target(&self, bounds: Rectangle, insert_x: Vec<f32>, scroll_offset: f32) {
        self.dock_state
            .borrow_mut()
            .tab_bar_targets
            .push(TabBarTarget {
                pane: self.pane_id,
                bounds,
                insert_x,
                scroll_offset,
            });
    }

    fn register_pane_bounds(&self, bounds: Rectangle) {
        self.dock_state
            .borrow_mut()
            .pane_bounds
            .push((self.pane_id, bounds));
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for TabDock<'_, Message, Theme, Renderer>
where
    Message: Clone + 'static,
    Theme: Catalog
        + button::Catalog
        + container::Catalog
        + iced_text::Catalog
        + Clone
        + PartialEq
        + 'static,
    Renderer: advanced::Renderer + advanced::text::Renderer + 'static,
    <Theme as button::Catalog>::Class<'static>:
        From<button::StyleFn<'static, Theme>>,
    <Theme as container::Catalog>::Class<'static>:
        From<container::StyleFn<'static, Theme>>,
    for<'b> <Theme as iced_text::Catalog>::Class<'b>:
        From<iced_text::StyleFn<'b, Theme>>,
{
    fn tag(&self) -> Tag {
        Tag::of::<TabDockState>()
    }

    fn state(&self) -> State {
        State::new(TabDockState)
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.tab_strip), Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        if tree.children.is_empty() {
            tree.children.push(Tree::new(&self.tab_strip));
            tree.children.push(Tree::new(&self.content));
            return;
        }
        tree.children[0].diff(&self.tab_strip);
        tree.children[1].diff(&self.content);
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
        let style = self.layout_style_or_default();
        let max = limits.max();
        let inset = pane_inset(&style);
        let inner_w = (max.width - 2.0 * inset).max(0.0);
        let inner_h = (max.height - 2.0 * inset).max(0.0);
        let tab_h = style.tab_bar.height;
        let content_h = (inner_h - tab_h).max(0.0);
        let mut y = inset;

        let tab_limits = layout::Limits::new(Size::ZERO, Size::new(inner_w, tab_h));
        let mut tab_node = compose::child_layout(
            &mut self.tab_strip,
            &mut tree.children[0],
            renderer,
            &tab_limits,
        );
        tab_node.move_to_mut((inset, y));
        y += tab_h;

        let content_limits = layout::Limits::new(Size::ZERO, Size::new(inner_w, content_h));
        let mut content_node = compose::child_layout(
            &mut self.content,
            &mut tree.children[1],
            renderer,
            &content_limits,
        );
        content_node.move_to_mut((inset, y));

        layout::Node::with_children(
            Size::new(max.width, max.height),
            vec![tab_node, content_node],
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
        self.register_pane_bounds(layout.bounds());

        let mut dock_style = self.layout_style(theme);
        dock_style.sync_tab_appearance();

        let pane_bounds = layout.bounds();
        let window = &dock_style.window;
        let is_focused = self.dock_state.borrow().focused_pane == Some(self.pane_id);
        let border = if is_focused {
            window.focused_border.unwrap_or(window.border)
        } else {
            window.border
        };

        renderer.fill_quad(
            renderer::Quad {
                bounds: pane_bounds,
                border,
                ..renderer::Quad::default()
            },
            window.background,
        );

        if let Some(tab_layout) = layout.children().next() {
            compose::child_draw(
                &self.tab_strip,
                &tree.children[0],
                renderer,
                theme,
                style,
                tab_layout,
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

        let drag_session = self.dock_state.borrow().drag;
        let show_overlay = self.is_dragging(tree);

        if show_overlay {
            if let Some(content_layout) = layout.children().nth(1) {
                let bounds = content_layout.bounds();
                let show_content_overlay = drag_session.is_some_and(|s| {
                    s.tab_insert.is_none() && s.hover_target == Some(self.pane_id)
                });
                let zone = show_content_overlay
                    .then(|| {
                        drag_session
                            .and_then(|s| s.operation)
                            .and_then(|_| cursor.position())
                            .and_then(|point| {
                                DockManager::hit_test_drop_zone(
                                    bounds,
                                    point,
                                    self.drop_edge_fraction,
                                )
                            })
                            .or_else(|| {
                                let point = cursor.position_over(bounds)?;
                                DockManager::hit_test_drop_zone(
                                    bounds,
                                    point,
                                    self.drop_edge_fraction,
                                )
                            })
                    })
                    .flatten();

                if let Some(zone) = zone {
                    let highlight = dock_style.drop_overlay.color;
                    let zone_bounds = drop_zone_rect(bounds, zone, self.drop_edge_fraction);
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
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let dragging = self.is_dragging(tree);

        let is_picked = self
            .dock_state
            .borrow()
            .drag
            .is_some_and(|session| session.source_pane == self.pane_id);

        if let Some(content_layout) = layout.children().nth(1) {
            self.register_drop_target(content_layout.bounds());
        }

        if let Some(tab_layout) = layout.children().next() {
            let suppress_hover = self.dock_state.borrow().drag.is_some();
            if tab_strip::set_suppress_hover::<Theme>(&mut tree.children[0], suppress_hover) {
                shell.request_redraw();
            }

            if let Some(row_layout) = tab_layout.children().next() {
                let scroll = tab_strip::scroll_offset::<Theme>(&tree.children[0]);
                let insert_x = tab_strip::build_insert_x(&row_layout);
                self.register_tab_bar_target(tab_layout.bounds(), insert_x, scroll);
            }

            compose::child_update(
                &mut self.tab_strip,
                &mut tree.children[0],
                event,
                tab_layout,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
            tab_strip::sync_hover_in_tree::<_, Theme>(
                &mut tree.children[0],
                tab_layout.bounds(),
                cursor,
                self.tab_bar_scrollbar_hide_delay,
                self.tab_bar_show_scrollbar,
                shell,
            );

            let marker_index = self.dock_state.borrow().drag.and_then(|session| {
                let (pane, index) = session.tab_insert?;
                (pane == self.pane_id
                    && !tab_insert_is_noop(session, self.pane_id, index, &self.tabs))
                .then_some(index)
            });
            if tab_strip::set_insert_marker_index::<Theme>(&mut tree.children[0], marker_index) {
                shell.request_redraw();
            }
        } else {
            tab_strip::set_insert_marker_index::<Theme>(&mut tree.children[0], None);
            tab_strip::set_suppress_hover::<Theme>(&mut tree.children[0], false);
        }
        if let Some(content_layout) = layout.children().nth(1) {
            if !is_picked {
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

            if !dragging {
                match event {
                    Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                    | Event::Touch(touch::Event::FingerPressed { .. }) => {
                        let bounds = content_layout.bounds();
                        if cursor.position_over(bounds).is_some() {
                            shell.capture_event();
                            shell.publish((self.on_event)(DockAction::PaneFocused {
                                pane: self.pane_id,
                                panel: Some(self.active_tab),
                            }));
                            shell.request_redraw();
                        }
                    }
                    _ => {}
                }
            }
        }

        if matches!(
            event,
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
        ) && self.dock_state.borrow().drag.is_some()
        {
            if let Some(pos) = cursor.position() {
                shell.publish((self.on_event)(DockAction::Tab(TabAction::DragEnded {
                    cursor: pos,
                })));
            } else {
                shell.publish((self.on_event)(DockAction::Tab(TabAction::DragCancelled)));
            }
            shell.invalidate_layout();
            shell.invalidate_widgets();
            shell.request_redraw();
        }

        if dragging {
            if let Event::Mouse(mouse::Event::CursorMoved { .. }) = event {
                if let Some(pos) = cursor.position() {
                    shell.publish((self.on_event)(DockAction::Tab(TabAction::DragMoved {
                        cursor: pos,
                    })));
                    shell.request_redraw();
                }
            }
        }

        tab_strip::schedule_hide_redraw::<_, Theme>(&tree.children[0], shell);
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        if self.dock_state.borrow().drag.is_some()
            || tab_strip::is_tab_drag_active::<Theme>(tree.children.first())
        {
            return mouse::Interaction::Grab;
        }

        let mut interaction = mouse::Interaction::None;
        if let Some(tab_layout) = layout.children().next() {
            interaction = interaction.max(compose::child_mouse_interaction(
                &self.tab_strip,
                &tree.children[0],
                tab_layout,
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
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        if let Some(tab_layout) = layout.children().next() {
            compose::child_operate(
                &mut self.tab_strip,
                &mut tree.children[0],
                tab_layout,
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

impl<'a, Message, Theme, Renderer> From<TabDock<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: Clone + 'static,
    Theme: Catalog
        + button::Catalog
        + container::Catalog
        + iced_text::Catalog
        + Clone
        + PartialEq
        + 'static,
    Renderer: advanced::Renderer + advanced::text::Renderer + 'static,
    <Theme as button::Catalog>::Class<'static>:
        From<button::StyleFn<'static, Theme>>,
    <Theme as container::Catalog>::Class<'static>:
        From<container::StyleFn<'static, Theme>>,
    for<'b> <Theme as iced_text::Catalog>::Class<'b>:
        From<iced_text::StyleFn<'b, Theme>>,
{
    fn from(widget: TabDock<'a, Message, Theme, Renderer>) -> Self {
        Element::new(widget)
    }
}
