//! Demo: complex IDE layout with splitters, tabs, and drag-dock.

use std::cell::RefCell;
use std::rc::Rc;

use iced::widget::{column, container, text};
use iced::{application, Color, Element, Length, Size, Task, Theme};

use iced_dock::{
    apply_message, dock, Axis, ContentKey, DockMessage, DockStyle, DockWidgetState, Factory,
    Layout, NodeKind,
};

fn complex_ide_layout(layout: &mut Layout) -> Result<(), ()> {
    let factory = Factory;

    let p_main = factory.insert_panel(layout, "main", "main.rs", ContentKey(0));
    let p_lib = factory.insert_panel(layout, "lib", "lib.rs", ContentKey(1));
    let pane_left_top = factory.create_pane(layout);
    factory.add_panel_to_pane(layout, pane_left_top, p_main)?;
    factory.add_panel_to_pane(layout, pane_left_top, p_lib)?;

    let p_prev = factory.insert_panel(layout, "preview", "preview", ContentKey(2));
    let pane_left_bot = factory.create_pane(layout);
    factory.add_panel_to_pane(layout, pane_left_bot, p_prev)?;

    let left_col = factory.create_proportional(
        layout,
        Axis::Vertical,
        vec![pane_left_top, pane_left_bot],
    );
    if let Some(NodeKind::Proportional(ref mut pg)) =
        layout.get_mut(left_col).map(|e| &mut e.kind)
    {
        pg.proportions = vec![0.55, 0.45];
    }

    let p_prop = factory.insert_panel(layout, "props", "Properties", ContentKey(10));
    let p_out = factory.insert_panel(layout, "output", "Output", ContentKey(11));
    let pane_right_top = factory.create_pane(layout);
    factory.add_panel_to_pane(layout, pane_right_top, p_prop)?;
    factory.add_panel_to_pane(layout, pane_right_top, p_out)?;

    let p_exp = factory.insert_panel(layout, "explorer", "Explorer", ContentKey(12));
    let p_srch = factory.insert_panel(layout, "search", "Search", ContentKey(13));
    let pane_right_bot = factory.create_pane(layout);
    factory.add_panel_to_pane(layout, pane_right_bot, p_exp)?;
    factory.add_panel_to_pane(layout, pane_right_bot, p_srch)?;

    let right_col = factory.create_proportional(
        layout,
        Axis::Vertical,
        vec![pane_right_top, pane_right_bot],
    );
    if let Some(NodeKind::Proportional(ref mut pg)) =
        layout.get_mut(right_col).map(|e| &mut e.kind)
    {
        pg.proportions = vec![0.5, 0.5];
    }

    let main = factory.create_proportional(layout, Axis::Horizontal, vec![left_col, right_col]);
    if let Some(NodeKind::Proportional(ref mut pg)) = layout.get_mut(main).map(|e| &mut e.kind) {
        pg.proportions = vec![0.72, 0.28];
    }

    layout.set_root_child(Some(main));
    Ok(())
}

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

struct App {
    dock_state: Rc<RefCell<DockWidgetState>>,
}

impl Default for App {
    fn default() -> Self {
        let mut state = DockWidgetState::default();
        complex_ide_layout(&mut state.layout).expect("complex_ide_layout seed");
        Self {
            dock_state: Rc::new(RefCell::new(state)),
        }
    }
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
    let dock_style = DockStyle::from_theme(&Theme::Dark);
    let window_background = dock_style.background.color;

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
    .style(move |_| container::Style {
        background: Some(window_background.into()),
        ..Default::default()
    })
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
