use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::{Operation, Tree};
use iced::advanced::{Clipboard, Shell};
use iced::mouse::{self, Cursor};
use iced::{Element, Event, Rectangle, Size};

pub fn child_layout<Message, Theme, Renderer>(
    child: &mut Element<'_, Message, Theme, Renderer>,
    tree: &mut Tree,
    renderer: &Renderer,
    limits: &layout::Limits,
) -> layout::Node
where
    Renderer: iced::advanced::Renderer,
{
    child.as_widget_mut().layout(tree, renderer, limits)
}

pub fn child_update<Message, Theme, Renderer>(
    child: &mut Element<'_, Message, Theme, Renderer>,
    tree: &mut Tree,
    event: &Event,
    layout: Layout<'_>,
    cursor: Cursor,
    renderer: &Renderer,
    clipboard: &mut dyn Clipboard,
    shell: &mut Shell<'_, Message>,
    viewport: &Rectangle,
) where
    Renderer: iced::advanced::Renderer,
{
    child.as_widget_mut().update(
        tree, event, layout, cursor, renderer, clipboard, shell, viewport,
    );
}

pub fn child_draw<Message, Theme, Renderer>(
    child: &Element<'_, Message, Theme, Renderer>,
    tree: &Tree,
    renderer: &mut Renderer,
    theme: &Theme,
    style: &renderer::Style,
    layout: Layout<'_>,
    cursor: Cursor,
    viewport: &Rectangle,
) where
    Renderer: iced::advanced::Renderer,
{
    child
        .as_widget()
        .draw(tree, renderer, theme, style, layout, cursor, viewport);
}

pub fn child_mouse_interaction<Message, Theme, Renderer>(
    child: &Element<'_, Message, Theme, Renderer>,
    tree: &Tree,
    layout: Layout<'_>,
    cursor: Cursor,
    viewport: &Rectangle,
    renderer: &Renderer,
) -> mouse::Interaction
where
    Renderer: iced::advanced::Renderer,
{
    child
        .as_widget()
        .mouse_interaction(tree, layout, cursor, viewport, renderer)
}

pub fn child_operate<Message, Theme, Renderer>(
    child: &mut Element<'_, Message, Theme, Renderer>,
    tree: &mut Tree,
    layout: Layout<'_>,
    renderer: &Renderer,
    operation: &mut dyn Operation,
) where
    Renderer: iced::advanced::Renderer,
{
    child
        .as_widget_mut()
        .operate(tree, layout, renderer, operation);
}
