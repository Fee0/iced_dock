//! Demo: complex IDE layout with splitters, tabs, and drag-dock.

use std::cell::RefCell;
use std::rc::Rc;

use iced::widget::{column, container, text};
use iced::{application, Color, Element, Length, Size, Task, Theme};

use iced_dock::{apply_message, dock, ContentKey, DockMessage, DockStyle, DockWidgetState};

fn main() -> iced::Result {
    application(App::default, update, view)
        .title("iced_dock — minimal")
        .theme(Theme::Dark)
        .window(iced::window::Settings {
            size: Size::new(1200.0, 800.0),
            ..Default::default()
        })
        .run()
}

#[derive(Default)]
struct App {
    dock_state: Rc<RefCell<DockWidgetState>>,
}

#[derive(Debug, Clone)]
enum Message {
    Dock(DockMessage),
}

fn update(app: &mut App, message: Message) -> Task<Message> {
    let Message::Dock(msg) = message;
    let _ = apply_message(&app.dock_state, msg);
    Task::none()
}

fn view(app: &App) -> Element<'_, Message> {
    container(
        dock::<Message>()
            .state(app.dock_state.clone())
            .on_event(Message::Dock)
            .content(panel)
            .style(|theme| DockStyle::from_theme(theme))
            .build(),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(10)
    .into()
}

fn panel(key: ContentKey) -> Element<'static, Message> {
    let (label, hint) = match key.0 {
        0 => ("main.rs", "Editor"),
        1 => ("lib.rs", "Editor"),
        2 => ("preview", "Preview"),
        10 => ("Properties", "Sidebar"),
        11 => ("Output", "Panel"),
        12 => ("Explorer", "Sidebar"),
        13 => ("Search", "Sidebar"),
        n => {
            return text(format!("Unknown pane {n}")).into();
        }
    };

    let fg = Color::from_rgb(0.78, 0.78, 0.82);
    let muted = Color::from_rgb(0.45, 0.45, 0.5);

    container(
        column![
            text(label).size(16).color(fg),
            text(hint).size(12).color(muted),
        ]
        .spacing(6)
        .padding([20, 24]),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .style(|_| container::Style {
        background: Some(Color::from_rgb(0.145, 0.145, 0.157).into()),
        ..Default::default()
    })
    .into()
}
