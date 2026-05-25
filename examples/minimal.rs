//! Demo: complex IDE layout with splitters, tabs, and drag-dock.

use iced::keyboard::{self, Key, Modifiers};
use iced::widget::{column, container, text};
use iced::{application, Element, Length, Size, Subscription, Task, Theme};
use iced_dock::{
    dock, horizontal, panel as tab, tabs, vertical, ContentKey, Direction, DockEvent, DockSession,
    LayoutTree,
};

fn demo_layout() -> LayoutTree {
    horizontal([
        vertical([
            tabs([
                tab("main", "main.rs", ContentKey(0)),
                tab("lib", "lib.rs", ContentKey(1)),
                tab("mod_a", "mod_a.rs", ContentKey(3)),
                tab("mod_b", "mod_b.rs", ContentKey(4)),
                tab("mod_c", "mod_c.rs", ContentKey(5)),
                tab("mod_d", "mod_d.rs", ContentKey(6)),
                tab("mod_e", "mod_e.rs", ContentKey(7)),
                tab("mod_f", "mod_f.rs", ContentKey(8)),
            ])
            .active("main"),
            tabs([tab("preview", "preview", ContentKey(2))]),
        ])
        .weights([0.55, 0.45]),
        vertical([
            tabs([
                tab("props", "Properties", ContentKey(10)),
                tab("output", "Output", ContentKey(11)),
            ]),
            tabs([
                tab("explorer", "Explorer", ContentKey(12)),
                tab("search", "Search", ContentKey(13)),
            ]),
        ])
        .weights([0.5, 0.5]),
    ])
    .weights([0.72, 0.28])
}

fn main() -> iced::Result {
    application(App::new, update, view)
        .title("iced_dock — minimal")
        .subscription(subscription)
        .window(iced::window::Settings {
            size: Size::new(1200.0, 800.0),
            ..Default::default()
        })
        .theme(Theme::Dark)
        .run()
}

struct App {
    dock: DockSession,
}

impl App {
    fn new() -> Self {
        Self {
            dock: DockSession::from_tree(demo_layout()).expect("failed to build demo layout"),
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    Dock(DockEvent),
    FocusAdjacent(Direction),
}

fn subscription(_app: &App) -> Subscription<Message> {
    keyboard::listen().filter_map(|event| {
        let keyboard::Event::KeyPressed { key, modifiers, .. } = event else {
            return None;
        };
        if !modifiers.contains(Modifiers::CTRL) {
            return None;
        }
        let direction = match key {
            Key::Named(keyboard::key::Named::ArrowLeft) => Direction::Left,
            Key::Named(keyboard::key::Named::ArrowRight) => Direction::Right,
            Key::Named(keyboard::key::Named::ArrowUp) => Direction::Up,
            Key::Named(keyboard::key::Named::ArrowDown) => Direction::Down,
            _ => return None,
        };
        Some(Message::FocusAdjacent(direction))
    })
}

fn update(app: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::Dock(_event) => {
            // Layout mutations are applied inside the dock widget; observe events here only.
        }
        Message::FocusAdjacent(direction) => {
            app.dock.focus_adjacent(direction);
        }
    }
    Task::none()
}

fn view(app: &App) -> Element<'_, Message> {
    container(
        dock()
            .state(app.dock.state())
            .on_event(Message::Dock)
            .content(panel)
            .min_pane_width(200.0)
            .min_pane_height(120.0)
            .tab_bar_show_scrollbar(false)
            .build(),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(10)
    .into()
}

fn panel(key: ContentKey) -> Element<'static, Message> {
    let body: Element<'static, Message> = match key.0 {
        10 => panel_body("Properties", "Panel"),
        0 => panel_body(
            "main.rs",
            "Editor — click to focus pane, Ctrl+Arrow to move focus",
        ),
        1 => panel_body("lib.rs", "Editor"),
        2 => panel_body("preview", "Preview"),
        11 => panel_body("Output", "Panel"),
        12 => panel_body("Explorer", "Sidebar"),
        13 => panel_body("Search", "Sidebar"),
        3 => panel_body("mod_a.rs", "Editor"),
        4 => panel_body("mod_b.rs", "Editor"),
        5 => panel_body("mod_c.rs", "Editor"),
        6 => panel_body("mod_d.rs", "Editor"),
        7 => panel_body("mod_e.rs", "Editor"),
        8 => panel_body("mod_f.rs", "Editor"),
        n => return text(format!("Unknown pane {n}")).into(),
    };

    container(body)
        .padding([20, 24])
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

fn panel_body(label: &'static str, hint: &'static str) -> Element<'static, Message> {
    column![
        text(label).size(16),
        text(hint).size(12).style(text::secondary),
    ]
    .spacing(6)
    .into()
}
