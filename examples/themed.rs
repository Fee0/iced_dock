//! Demo: per-pane theming with `PaneContent` style overrides.
//!
//! The sidebar panes use a warm accent theme while the editor/preview panes
//! use the default palette-derived style. Drag tabs between panes to see
//! styles follow their host pane, not the tab.

use iced::keyboard::{self, Key, Modifiers};
use iced::widget::{column, container, text};
use iced::{application, Border, Color, Element, Length, Size, Subscription, Task};

use iced_dock::{
    dock, horizontal, panel as tab, tabs, vertical, ContentKey, Direction, DockEvent, DockSession,
    DockStyle, LayoutTree, PaneContent,
};

const SIDEBAR_KEYS: &[u32] = &[10, 11, 12, 13];

fn demo_layout() -> LayoutTree {
    horizontal([
        vertical([
            tabs([
                tab("main", "main.rs", ContentKey(0)),
                tab("lib", "lib.rs", ContentKey(1)),
                tab("mod_a", "mod_a.rs", ContentKey(3)),
                tab("mod_b", "mod_b.rs", ContentKey(4)),
            ])
            .active("main"),
            tabs([tab("preview", "Preview", ContentKey(2))]),
        ])
        .weights([0.6, 0.4]),
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
    .weights([0.7, 0.3])
}

fn sidebar_style(theme: &iced::Theme) -> DockStyle {
    let warm = Color::from_rgb(0.56, 0.34, 0.13);
    let warm_strong = Color::from_rgb(0.72, 0.44, 0.16);

    let mut style = iced_dock::default(theme);

    style.tab.active_accent = warm;
    style.window.focused_border = Some(Border {
        color: warm_strong,
        ..style.window.border
    });
    style.splitter.hover_color = warm;
    style.splitter.drag_color = warm_strong;

    style
}

fn main() -> iced::Result {
    application(App::new, update, view)
        .title("iced_dock — per-pane theming")
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
            .content_styled(panel_content)
            .min_pane_width(180.0)
            .min_pane_height(100.0)
            .tab_bar_show_scrollbar(false)
            .build(),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(10)
    .into()
}

fn panel_content(key: ContentKey) -> PaneContent<'static, Message> {
    let is_sidebar = SIDEBAR_KEYS.contains(&key.0);

    let (fg, muted) = if is_sidebar {
        (
            Color::from_rgb(0.82, 0.74, 0.62),
            Color::from_rgb(0.55, 0.48, 0.38),
        )
    } else {
        (
            Color::from_rgb(0.78, 0.78, 0.82),
            Color::from_rgb(0.45, 0.45, 0.5),
        )
    };

    let body: Element<'static, Message> = match key.0 {
        0 => panel_body("main.rs", "Editor — per-pane theming demo", fg, muted),
        1 => panel_body("lib.rs", "Editor", fg, muted),
        2 => panel_body("Preview", "Default palette style", fg, muted),
        3 => panel_body("mod_a.rs", "Editor", fg, muted),
        4 => panel_body("mod_b.rs", "Editor", fg, muted),
        10 => panel_body("Properties", "Sidebar — warm accent style", fg, muted),
        11 => panel_body("Output", "Sidebar — warm accent style", fg, muted),
        12 => panel_body("Explorer", "Sidebar — warm accent style", fg, muted),
        13 => panel_body("Search", "Sidebar — warm accent style", fg, muted),
        n => return PaneContent::new(text(format!("Unknown pane {n}"))),
    };

    let element: Element<'static, Message> = container(body)
        .padding([20, 24])
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into();

    if is_sidebar {
        PaneContent::new(element).style(sidebar_style)
    } else {
        PaneContent::new(element)
    }
}

fn panel_body(
    label: &'static str,
    hint: &'static str,
    fg: Color,
    muted: Color,
) -> Element<'static, Message> {
    column![
        text(label).size(16).color(fg),
        text(hint).size(12).color(muted),
    ]
    .spacing(6)
    .into()
}
