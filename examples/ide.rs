//! Demo: classical IDE layout — left sidebar, editor tabs, right sidebar, bottom panel.

use std::collections::HashSet;

use iced::keyboard::{self, Key};
use iced::widget::{button, column, container, text};
use iced::{application, Color, Element, Length, Size, Subscription, Task, Theme};
use iced_dock::{
    dock, horizontal, model::NodeKind, panel as tab, tabs, vertical, Direction, DockEvent,
    DockSession, DockStyle, InitialFocus, LayoutTree,
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
                tab("search", "Search", Content::Search),
            ])
            .active("explorer")
            .group("tools"),
            // Main editor — documents group
            tabs([
                tab("main", "main.rs", Content::MainRs),
                tab("lib", "lib.rs", Content::LibRs),
                tab("mod_a", "module_alpha.rs", Content::ModA),
                tab("mod_b", "module_alpha_beta.rs", Content::ModB),
                tab("mod_c", "module_alpha_gamma.rs", Content::ModC),
                tab("mod_d", "module_alpha_delta.rs", Content::ModD),
                tab("cargo", "Cargo.toml", Content::CargoToml),
            ])
            .persistent(true)
            .active("main")
            .group("documents"),
            // Right sidebar — tools group
            tabs([
                tab("outline", "Outline", Content::Outline),
                tab("properties", "Properties", Content::Properties),
            ])
            .active("outline")
            .group("tools"),
        ])
        .weights([0.18, 0.62, 0.20]),
        // Bottom panel — tools group
        tabs([
            tab("terminal", "Terminal", Content::Terminal),
            tab("output", "Output", Content::Output),
            tab("problems", "Problems", Content::Problems),
            tab("debug", "Debug Console", Content::Debug),
        ])
        .active("terminal")
        .group("tools"),
    ])
    .weights([0.75, 0.25])
}

fn panel_id(key: Content) -> &'static str {
    match key {
        Content::Explorer => "explorer",
        Content::Search => "search",
        Content::MainRs => "main",
        Content::LibRs => "lib",
        Content::ModA => "mod_a",
        Content::ModB => "mod_b",
        Content::ModC => "mod_c",
        Content::ModD => "mod_d",
        Content::CargoToml => "cargo",
        Content::Outline => "outline",
        Content::Properties => "properties",
        Content::Terminal => "terminal",
        Content::Output => "output",
        Content::Problems => "problems",
        Content::Debug => "debug",
    }
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
    modified: HashSet<Content>,
}

impl App {
    fn new() -> Self {
        Self {
            dock: DockSession::from_tree_with_focus(
                demo_layout(),
                InitialFocus::NamedPanel("main".into()),
            )
            .expect("failed to build demo layout"),
            modified: HashSet::new(),
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    Dock(DockEvent<Content>),
    FocusAdjacent(Direction),
    MoveActivePanel(Direction),
    SplitActivePanel(Direction),
    ToggleModified(Content),
}

fn subscription(_app: &App) -> Subscription<Message> {
    keyboard::listen().filter_map(|event| {
        let keyboard::Event::KeyPressed { key, modifiers, .. } = event else {
            return None;
        };
        if !modifiers.command() && !modifiers.alt() {
            return None;
        }
        let direction = match key {
            Key::Named(keyboard::key::Named::ArrowLeft) => Direction::Left,
            Key::Named(keyboard::key::Named::ArrowRight) => Direction::Right,
            Key::Named(keyboard::key::Named::ArrowUp) => Direction::Up,
            Key::Named(keyboard::key::Named::ArrowDown) => Direction::Down,
            _ => return None,
        };
        if modifiers.command() && modifiers.shift() {
            Some(Message::SplitActivePanel(direction))
        } else if modifiers.alt() {
            Some(Message::MoveActivePanel(direction))
        } else {
            Some(Message::FocusAdjacent(direction))
        }
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
        Message::MoveActivePanel(direction) => {
            app.dock.move_active_panel_adjacent(direction);
        }
        Message::SplitActivePanel(direction) => {
            app.dock.split_active_panel(direction);
        }
        Message::ToggleModified(key) => {
            let is_modified = !app.modified.contains(&key);
            if is_modified {
                app.modified.insert(key);
            } else {
                app.modified.remove(&key);
            }
            if let Some(node_id) = app.dock.panel_node(panel_id(key)) {
                let state_rc = app.dock.state();
                let mut state = state_rc.borrow_mut();
                if let Some(NodeKind::Panel(ref mut panel)) =
                    state.layout.get_mut(node_id).map(|e| &mut e.kind)
                {
                    panel.is_modified = is_modified;
                }
                state.layout_dirty = true;
            }
        }
    }
    Task::none()
}

fn view(app: &App) -> Element<'_, Message> {
    let modified = app.modified.clone();
    container(
        dock()
            .state(app.dock.state())
            .on_event(Message::Dock)
            .content(move |key| panel_content(key, modified.contains(&key)))
            .style(|theme| {
                let mut style = DockStyle::from_palette(theme);
                style.tab.modified_background =
                    Some(Color::from_rgba(0.90, 0.55, 0.10, 0.30));
                style
            })
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

fn panel_content(key: Content, is_modified: bool) -> Element<'static, Message> {
    let (label, hint) = match key {
        Content::Explorer => ("Explorer", "File tree"),
        Content::Search => ("Search", "Workspace search"),
        Content::MainRs => (
            "main.rs",
            "Editor — ⌘/Ctrl+Arrow moves focus, Alt+Arrow moves tabs, ⌘/Ctrl+Shift+Arrow splits",
        ),
        Content::LibRs => ("lib.rs", "Editor"),
        Content::ModA => ("mod_a.rs", "Editor"),
        Content::ModB => ("mod_b.rs", "Editor"),
        Content::ModC => ("mod_c.rs", "Editor"),
        Content::ModD => ("mod_d.rs", "Editor"),
        Content::CargoToml => ("Cargo.toml", "Editor"),
        Content::Outline => ("Outline", "Symbol outline"),
        Content::Properties => ("Properties", "Item properties"),
        Content::Terminal => ("Terminal", "Integrated terminal"),
        Content::Output => ("Output", "Build & run output"),
        Content::Problems => ("Problems", "Errors and warnings"),
        Content::Debug => ("Debug Console", "Debugger output"),
    };

    let is_doc = matches!(
        key,
        Content::MainRs
            | Content::LibRs
            | Content::ModA
            | Content::ModB
            | Content::ModC
            | Content::ModD
            | Content::CargoToml
    );

    let mut col = column![
        text(label).size(15),
        text(hint).size(12).style(text::secondary),
    ]
    .spacing(6);

    if is_doc {
        let btn_label = if is_modified {
            "Mark as saved"
        } else {
            "Mark as modified"
        };
        col = col.push(
            button(text(btn_label))
                .on_press(Message::ToggleModified(key)),
        );
    }

    container(col)
        .padding([20, 24])
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}
