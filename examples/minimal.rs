//! Demo: classical IDE layout — left sidebar, editor tabs, right sidebar, bottom panel.

use iced::keyboard::{self, Key};
use iced::widget::{column, container, text};
use iced::{application, Element, Length, Size, Subscription, Task, Theme};
use iced_dock::{
    dock, horizontal, panel as tab, tabs, vertical, Direction, DockEvent, DockSession,
    InitialFocus, LayoutTree,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Content {
    Explorer,
    Search,
    MainRs,
    LibRs,
    ModA,
    ModB,
    ModC,
    ModD,
    CargoToml,
    Outline,
    Properties,
    Terminal,
    Output,
    Problems,
    Debug,
}

fn demo_layout() -> LayoutTree<Content> {
    vertical([
        horizontal([
            // Left sidebar — tools group
            tabs([
                tab("explorer", "Explorer", Content::Explorer),
                tab("search",   "Search",   Content::Search),
            ])
            .active("explorer")
            .group("tools"),

            // Main editor — documents group
            tabs([
                tab("main",    "main.rs",    Content::MainRs),
                tab("lib",     "lib.rs",     Content::LibRs),
                tab("mod_a",   "mod_a.rs",   Content::ModA),
                tab("mod_b",   "mod_b.rs",   Content::ModB),
                tab("mod_c",   "mod_c.rs",   Content::ModC),
                tab("mod_d",   "mod_d.rs",   Content::ModD),
                tab("cargo",   "Cargo.toml", Content::CargoToml),
            ])
            .active("main")
            .group("documents"),

            // Right sidebar — tools group
            tabs([
                tab("outline",    "Outline",    Content::Outline),
                tab("properties", "Properties", Content::Properties),
            ])
            .active("outline")
            .group("tools"),
        ])
        .weights([0.18, 0.62, 0.20]),

        // Bottom panel — tools group
        tabs([
            tab("terminal", "Terminal",      Content::Terminal),
            tab("output",   "Output",        Content::Output),
            tab("problems", "Problems",      Content::Problems),
            tab("debug",    "Debug Console", Content::Debug),
        ])
        .active("terminal")
        .group("tools"),
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
    dock: DockSession<Content>,
}

impl App {
    fn new() -> Self {
        Self {
            dock: DockSession::from_tree_with_focus(demo_layout(), InitialFocus::NamedPanel("main".into()))
                .expect("failed to build demo layout"),
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
        if !modifiers.command() {
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
        Message::Dock(_event) => {
            // listen for dock events here
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
            .content(panel_content)
            .min_pane_width(160.0)
            .min_pane_height(80.0)
            .tab_bar_show_scrollbar(true)
            .build(),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(10)
    .into()
}

fn panel_content(key: Content) -> Element<'static, Message> {
    let (label, hint) = match key {
        Content::Explorer   => ("Explorer",      "File tree"),
        Content::Search     => ("Search",        "Workspace search"),
        Content::MainRs     => ("main.rs",       "Editor — ⌘/Ctrl+Arrow to move focus between panes"),
        Content::LibRs      => ("lib.rs",        "Editor"),
        Content::ModA       => ("mod_a.rs",      "Editor"),
        Content::ModB       => ("mod_b.rs",      "Editor"),
        Content::ModC       => ("mod_c.rs",      "Editor"),
        Content::ModD       => ("mod_d.rs",      "Editor"),
        Content::CargoToml  => ("Cargo.toml",    "Editor"),
        Content::Outline    => ("Outline",       "Symbol outline"),
        Content::Properties => ("Properties",    "Item properties"),
        Content::Terminal   => ("Terminal",      "Integrated terminal"),
        Content::Output     => ("Output",        "Build & run output"),
        Content::Problems   => ("Problems",      "Errors and warnings"),
        Content::Debug      => ("Debug Console", "Debugger output"),
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
