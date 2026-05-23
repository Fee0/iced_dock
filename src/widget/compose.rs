use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::{Operation, Tree};
use iced::advanced::{Clipboard, Shell};
use iced::mouse::{self, Cursor};
use iced::{Element, Event, Rectangle, Size, Theme};

/// Delegate layout/update/draw to a single child [`Element`].
pub fn child_layout<Message>(
    child: &mut Element<'_, Message, Theme, iced::Renderer>,
    tree: &mut Tree,
    renderer: &iced::Renderer,
    limits: &layout::Limits,
) -> layout::Node
where
    Message: Clone,
{
    child.as_widget_mut().layout(tree, renderer, limits)
}

pub fn child_update<Message>(
    child: &mut Element<'_, Message, Theme, iced::Renderer>,
    tree: &mut Tree,
    event: &Event,
    layout: Layout<'_>,
    cursor: Cursor,
    renderer: &iced::Renderer,
    clipboard: &mut dyn Clipboard,
    shell: &mut Shell<'_, Message>,
    viewport: &Rectangle,
) where
    Message: Clone,
{
    child.as_widget_mut().update(
        tree, event, layout, cursor, renderer, clipboard, shell, viewport,
    );
}

pub fn child_draw<Message>(
    child: &Element<'_, Message, Theme, iced::Renderer>,
    tree: &Tree,
    renderer: &mut iced::Renderer,
    theme: &Theme,
    style: &renderer::Style,
    layout: Layout<'_>,
    cursor: Cursor,
    viewport: &Rectangle,
) where
    Message: Clone,
{
    child.as_widget().draw(tree, renderer, theme, style, layout, cursor, viewport);
}

pub fn child_mouse_interaction<Message>(
    child: &Element<'_, Message, Theme, iced::Renderer>,
    tree: &Tree,
    layout: Layout<'_>,
    cursor: Cursor,
    viewport: &Rectangle,
    renderer: &iced::Renderer,
) -> mouse::Interaction
where
    Message: Clone,
{
    child
        .as_widget()
        .mouse_interaction(tree, layout, cursor, viewport, renderer)
}

#[allow(dead_code)]
pub fn child_size<Message>(
    child: &Element<'_, Message, Theme, iced::Renderer>,
) -> Size<iced::Length>
where
    Message: Clone,
{
    child.as_widget().size()
}

pub fn child_operate<Message>(
    child: &mut Element<'_, Message, Theme, iced::Renderer>,
    tree: &mut Tree,
    layout: Layout<'_>,
    renderer: &iced::Renderer,
    operation: &mut dyn Operation,
) where
    Message: Clone,
{
    child
        .as_widget_mut()
        .operate(tree, layout, renderer, operation);
}
