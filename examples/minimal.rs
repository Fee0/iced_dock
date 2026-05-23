//! Demo: complex IDE layout with splitters, tabs, and drag-dock.

use std::cell::RefCell;
use std::rc::Rc;

use iced::widget::{column, container, text};
use iced::{application, Color, Element, Length, Size, Task, Theme};

use iced_dock::{apply_message, dock, ContentKey, DockMessage, DockWidgetState};

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
            .build(),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn panel(key: ContentKey) -> Element<'static, Message> {
    let (label, color) = match key.0 {
        0 => ("main.rs", Color::from_rgb(0.15, 0.2, 0.35)),
        1 => ("lib.rs", Color::from_rgb(0.12, 0.28, 0.22)),
        2 => ("preview", Color::from_rgb(0.25, 0.18, 0.3)),
        10 => ("Properties", Color::from_rgb(0.22, 0.22, 0.18)),
        11 => ("Output", Color::from_rgb(0.18, 0.2, 0.25)),
        12 => ("Explorer", Color::from_rgb(0.2, 0.15, 0.15)),
        13 => ("Search", Color::from_rgb(0.15, 0.18, 0.28)),
        n => {
            return text(format!("Unknown pane {n}")).into();
        }
    };

    container(
        column![text(label).size(20)]
            .spacing(8)
            .padding(16),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .style(move |_| container::Style {
        background: Some(color.into()),
        ..Default::default()
    })
    .into()
}
