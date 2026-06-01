//! Regression tests for tab switches when pane content uses nested fill containers
//! and different widget trees per tab (the Hexcraft welcome ↔ file scenario).
//!
//! Before the fix, `Dock::update` called `rebuild_root` while `layout_dirty`, which
//! desynced the widget tree from the cached layout and could panic in iced's
//! `container::mouse_interaction` on the same frame.

use iced::widget::{container, text, Column};
use iced::{Element, Length, Point, Size};
use iced_dock::{dock, panel, tabs, DockSession};
use iced_test::Simulator;

#[derive(Debug, Clone)]
enum Message {
    Empty,
}

fn welcome_file_session() -> DockSession<u32> {
    DockSession::from_tree(
        tabs([
            panel("welcome", "Welcome", 0u32),
            panel("file", "Document", 1u32),
        ])
        .active("welcome"),
    )
    .expect("valid layout")
}

fn panel_content(key: u32) -> Element<'static, Message> {
    let inner: Element<'static, Message> = match key {
        0 => Column::new()
            .push(text("Content:welcome"))
            .push(text("New File"))
            .push(text("Open File"))
            .into(),
        1 => text("Content:file").into(),
        _ => text("Content:unknown").into(),
    };

    container(inner)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn view(session: &DockSession<u32>) -> Element<'_, Message> {
    container(
        dock::<u32, Message, iced::Theme, iced::Renderer>()
            .state(session.state())
            .on_event(|_| Message::Empty)
            .content(|key| panel_content(key))
            .build(),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

#[test]
fn tab_switch_between_welcome_and_file_with_cursor_over_content() {
    let session = welcome_file_session();
    let mut ui = Simulator::with_size(Default::default(), Size::new(1024.0, 768.0), view(&session));

    ui.point_at(Point::new(512.0, 400.0));

    let _ = ui.click("Document");
    ui.find("Content:file")
        .expect("file panel content should be visible after tab switch");

    let _ = ui.click("Welcome");
    ui.find("Content:welcome")
        .expect("welcome panel content should be visible after tab switch");

    let _ = ui.click("Document");
    ui.find("Content:file")
        .expect("file panel content should be visible after second tab switch");
}
