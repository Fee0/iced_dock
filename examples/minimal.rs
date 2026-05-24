//! Demo: complex IDE layout with splitters, tabs, and drag-dock.

use iced::keyboard::{self, Key, Modifiers};
use iced::widget::{column, container, text};
use iced::{application, Color, Element, Length, Size, Subscription, Task, Theme};

use iced_dock::{
    dock, horizontal, panel as tab, tabs, vertical, ContentKey, Direction, DockMessage,
    DockSession, DockStyle, LayoutTree,
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
        .theme(Theme::Dark)
        .subscription(subscription)
        .window(iced::window::Settings {
            size: Size::new(1200.0, 800.0),
            ..Default::default()
        })
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
    Dock(DockMessage),
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
        Message::Dock(msg) => {
            let _ = app.dock.apply_message(msg);
        }
        Message::FocusAdjacent(direction) => {
            app.dock.focus_adjacent(direction);
        }
    }
    Task::none()
}

fn view(app: &App) -> Element<'_, Message> {
    let dock_style = DockStyle::from_theme(&Theme::Dark);
    let window_background = dock_style.background.color;

    container(
        dock::<Message>()
            .state(app.dock.state())
            .on_event(Message::Dock)
            .content(panel)
            .min_pane_width(200.0)
            .min_pane_height(120.0)
            .tab_bar_show_scrollbar(false)
            .style(|theme| DockStyle::from_theme(theme))
            .build(),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(10)
    .style(move |_| container::Style {
        background: Some(window_background.into()),
        ..Default::default()
    })
    .into()
}

fn panel(key: ContentKey) -> Element<'static, Message> {
    let (label, hint) = match key.0 {
        0 => ("main.rs", "Editor — click to focus pane, Ctrl+Arrow to move focus"),
        1 => ("lib.rs", "Editor"),
        2 => ("preview", "Preview"),
        10 => ("Properties", "Sidebar"),
        11 => ("Output", "Panel"),
        12 => ("Explorer", "Sidebar"),
        13 => ("Search", "Sidebar"),
        3 => ("mod_a.rs", "Editor"),
        4 => ("mod_b.rs", "Editor"),
        5 => ("mod_c.rs", "Editor"),
        6 => ("mod_d.rs", "Editor"),
        7 => ("mod_e.rs", "Editor"),
        8 => ("mod_f.rs", "Editor"),
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
