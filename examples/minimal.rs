//! Demo: classical IDE layout — left sidebar, editor tabs, right sidebar, bottom panel.

use iced::keyboard::{self, Key, Modifiers};
use iced::widget::{column, container, text};
use iced::{application, Element, Length, Size, Subscription, Task, Theme};
use iced_dock::{
    dock, horizontal, panel as tab, tabs, vertical, ContentKey, Direction, DockEvent, DockSession,
    LayoutTree,
};

fn demo_layout() -> LayoutTree {
    vertical([
        horizontal([
            // Left sidebar
            tabs([
                tab("explorer", "Explorer", ContentKey(0)),
                tab("search",   "Search",   ContentKey(1)),
            ])
            .active("explorer"),

            // Main editor
            tabs([
                tab("main",    "main.rs",    ContentKey(10)),
                tab("lib",     "lib.rs",     ContentKey(11)),
                tab("mod_a",   "mod_a.rs",   ContentKey(12)),
                tab("mod_b",   "mod_b.rs",   ContentKey(13)),
                tab("mod_c",   "mod_c.rs",   ContentKey(14)),
                tab("mod_d",   "mod_d.rs",   ContentKey(15)),
                tab("cargo",   "Cargo.toml", ContentKey(16)),
            ])
            .active("main"),

            // Right sidebar
            tabs([
                tab("outline",    "Outline",    ContentKey(30)),
                tab("properties", "Properties", ContentKey(31)),
            ])
            .active("outline"),
        ])
        .weights([0.18, 0.62, 0.20]),

        // Bottom panel
        tabs([
            tab("terminal", "Terminal", ContentKey(20)),
            tab("output",   "Output",   ContentKey(21)),
            tab("problems", "Problems", ContentKey(22)),
            tab("debug",    "Debug Console", ContentKey(23)),
        ])
        .active("terminal"),
    ])
    .weights([0.75, 0.25])
}

fn main() -> iced::Result {
    application(App::new, update, view)
        .title("iced_dock — IDE layout")
        .subscription(subscription)
        .window(iced::window::Settings {
            size: Size::new(1280.0, 800.0),
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
            Key::Named(keyboard::key::Named::ArrowLeft)  => Direction::Left,
            Key::Named(keyboard::key::Named::ArrowRight) => Direction::Right,
            Key::Named(keyboard::key::Named::ArrowUp)    => Direction::Up,
            Key::Named(keyboard::key::Named::ArrowDown)  => Direction::Down,
            _ => return None,
        };
        Some(Message::FocusAdjacent(direction))
    })
}

fn update(app: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::Dock(_event) => {}
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
            .min_pane_width(160.0)
            .min_pane_height(80.0)
            .tab_bar_show_scrollbar(false)
            .build(),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(10)
    .into()
}

fn panel(key: ContentKey) -> Element<'static, Message> {
    let (label, hint) = match key.0 {
        0  => ("Explorer",      "File tree"),
        1  => ("Search",        "Workspace search"),
        2  => ("Git",           "Source control"),
        10 => ("main.rs",       "Editor — Ctrl+Arrow to move focus between panes"),
        11 => ("lib.rs",        "Editor"),
        12 => ("mod_a.rs",      "Editor"),
        13 => ("mod_b.rs",      "Editor"),
        14 => ("mod_c.rs",      "Editor"),
        15 => ("mod_d.rs",      "Editor"),
        16 => ("Cargo.toml",    "Editor"),
        20 => ("Terminal",      "Integrated terminal"),
        21 => ("Output",        "Build & run output"),
        22 => ("Problems",      "Errors and warnings"),
        23 => ("Debug Console", "Debugger output"),
        30 => ("Outline",       "Symbol outline"),
        31 => ("Properties",    "Item properties"),
        n  => return text(format!("Unknown panel {n}")).into(),
    };

    container(
        column![
            text(label).size(15),
            text(hint).size(12).style(text::secondary),
        ]
        .spacing(6),
    )
    .padding([20, 24])
    .width(Length::Fill)
    .height(Length::Fill)
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .into()
}
