//! Minimal docking demo: documents, tools, splits, tab close.

use iced::widget::{column, container, text};
use iced::{application, Element, Length, Task, Theme};

use iced_dock::{build_layout, ContentKey, DockMessage, DockState};

fn main() -> iced::Result {
    application(new, update, view)
        .title("iced_dock — minimal")
        .theme(theme)
        .run()
}

struct App {
    dock: DockState<Message>,
}

fn theme(_: &App) -> Theme {
    Theme::Dark
}

#[derive(Debug, Clone)]
enum Message {
    Dock(DockMessage),
}

fn new() -> (App, Task<Message>) {
    let mut dock = DockState::new(Message::Dock);
    dock.init_default_layout(
        vec![
            ("Document A".into(), ContentKey(0)),
            ("Document B".into(), ContentKey(1)),
        ],
        vec![
            ("Properties".into(), ContentKey(10)),
            ("Output".into(), ContentKey(11)),
        ],
    )
    .expect("layout");

    (App { dock }, Task::none())
}

fn update(app: &mut App, message: Message) -> Task<Message> {
    let Message::Dock(msg) = message;
    app.dock.update(msg);
    Task::none()
}

fn view(app: &App) -> Element<'_, Message> {
    let dock_element = match app.dock.layout.root_child() {
        Some(root) => build_layout(
            &app.dock.layout,
            app.dock.drag_session(),
            app.dock.on_message(),
            &content_panel,
            root,
        ),
        None => text("empty dock").into(),
    };

    column![
        text("iced_dock minimal example").size(14),
        container(dock_element)
            .width(Length::Fill)
            .height(Length::Fill),
    ]
    .spacing(8)
    .padding(8)
    .into()
}

fn content_panel(key: ContentKey) -> Element<'static, Message> {
    let label = match key.0 {
        0 => "Document A content",
        1 => "Document B content",
        10 => "Properties panel",
        11 => "Output panel",
        _ => "Panel",
    };
    container(
        column![
            text(label).size(18),
            text("Drag tabs to dock. Resize splits via handles.").size(12),
        ]
        .spacing(8),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(16)
    .into()
}
