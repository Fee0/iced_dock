//! Demo: custom close and overflow icons via `.close_icon()` and `.overflow_icon()`.
//!
//! The close buttons show a text "×" instead of the default SVG, and the
//! overflow chevron is replaced by "›". Resize the window narrow enough to
//! trigger the overflow button and see both icons at once.

use iced::widget::{center, column, container, text};
use iced::{application, Color, Element, Font, Length, Size, Task};
use iced_dock::{
    dock, horizontal, panel as tab, tabs, vertical, DockEvent, DockSession, LayoutTree,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Panel {
    Explorer,
    Search,
    Alpha,
    Beta,
    Gamma,
    Delta,
    Epsilon,
    Zeta,
    Terminal,
    Output,
}

fn demo_layout() -> LayoutTree<Panel> {
    vertical([
        horizontal([
            tabs([
                tab("explorer", "Explorer", Panel::Explorer),
                tab("search", "Search", Panel::Search),
            ])
            .active("explorer"),
            // Many tabs so the overflow button appears when the window is narrow
            tabs([
                tab("alpha", "alpha.rs", Panel::Alpha),
                tab("beta", "beta.rs", Panel::Beta),
                tab("gamma", "gamma.rs", Panel::Gamma),
                tab("delta", "delta.rs", Panel::Delta),
                tab("epsilon", "epsilon.rs", Panel::Epsilon),
                tab("zeta", "zeta.rs", Panel::Zeta),
            ])
            .active("alpha"),
        ])
        .weights([0.25, 0.75]),
        tabs([
            tab("terminal", "Terminal", Panel::Terminal),
            tab("output", "Output", Panel::Output),
        ])
        .active("terminal"),
    ])
    .weights([0.75, 0.25])
}

fn main() -> iced::Result {
    application(App::new, update, view)
        .title("iced_dock — custom icons")
        .window(iced::window::Settings {
            size: Size::new(1100.0, 720.0),
            ..Default::default()
        })
        .theme(|_app: &App| iced::Theme::Dark)
        .run()
}

struct App {
    dock: DockSession<Panel>,
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
    Dock(DockEvent<Panel>),
}

fn update(_app: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::Dock(_) => {}
    }
    Task::none()
}

fn view(app: &App) -> Element<'_, Message> {
    container(
        dock()
            .state(app.dock.state())
            .on_event(Message::Dock)
            .content(panel_content)
            .tab_bar_show_scrollbar(true)
            // Replace the default close SVG with a styled text glyph.
            .close_icon(close_icon)
            // Replace the default chevron SVG with a text glyph.
            .overflow_icon(overflow_icon)
            .build(),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(10)
    .into()
}

fn close_icon() -> Element<'static, Message> {
    center(
        text("×")
            .size(14)
            .font(Font::MONOSPACE)
            .color(Color::from_rgb(0.7, 0.7, 0.7)),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn overflow_icon() -> Element<'static, Message> {
    center(
        text("›")
            .size(18)
            .font(Font::MONOSPACE)
            .color(Color::from_rgb(0.7, 0.7, 0.7)),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn panel_content(key: Panel) -> Element<'static, Message> {
    let (title, hint) = match key {
        Panel::Explorer => (
            "Explorer",
            "Resize the window narrow to see the overflow icon (›)",
        ),
        Panel::Search => ("Search", ""),
        Panel::Alpha => (
            "alpha.rs",
            "Close buttons show × instead of the default SVG",
        ),
        Panel::Beta => ("beta.rs", ""),
        Panel::Gamma => ("gamma.rs", ""),
        Panel::Delta => ("delta.rs", ""),
        Panel::Epsilon => ("epsilon.rs", ""),
        Panel::Zeta => ("zeta.rs", ""),
        Panel::Terminal => ("Terminal", ""),
        Panel::Output => ("Output", ""),
    };

    container(
        column![
            text(title).size(15),
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
