//! Tab bar + content host.

use iced::alignment;
use iced::widget::{button, row, text};
use iced::{Element, Length, Padding, Point};

use crate::manager::DropZone;
use crate::model::{Layout as DockLayout, NodeId, NodeKind};

const TAB_HEIGHT: f32 = 28.0;

/// Messages from tab dock UI.
#[derive(Debug, Clone)]
pub enum TabMessage {
    Selected { group: NodeId, index: usize },
    Closed { group: NodeId, index: usize },
    DragStarted {
        group: NodeId,
        index: usize,
        point: Point,
    },
    DragMoved { x: f32, y: f32 },
    DragEnded {
        target: Option<NodeId>,
        zone: DropZone,
        x: f32,
        y: f32,
    },
}

fn tab_titles(layout: &DockLayout, group: NodeId) -> Vec<(String, bool)> {
    let Some(NodeKind::TabGroup(g)) = layout.kind(group) else {
        return Vec::new();
    };
    g.children
        .iter()
        .filter_map(|&id| {
            let title = match layout.kind(id)? {
                NodeKind::Document(m) | NodeKind::Tool(m) => m.title.clone(),
                _ => return None,
            };
            let can_close = match layout.kind(id)? {
                NodeKind::Document(m) | NodeKind::Tool(m) => m.can_close,
                _ => false,
            };
            Some((title, can_close))
        })
        .collect()
}

/// Custom tab dock built from iced widgets (tab bar + active content).
pub fn tab_dock_simple<'a, Message: Clone + 'static>(
    group: NodeId,
    layout: &'a DockLayout,
    active_content: Element<'a, Message>,
    on_message: impl Fn(TabMessage) -> Message + 'a,
) -> Element<'a, Message> {
    let titles = tab_titles(layout, group);
    let tabs: Vec<Element<'a, Message>> = titles
        .into_iter()
        .enumerate()
        .map(|(i, (title, can_close))| {
            let label: Element<'a, Message> = if can_close {
                row![
                    text(title).size(12),
                    button("×")
                        .padding(Padding::new(2.0))
                        .on_press(on_message(TabMessage::Closed { group, index: i })),
                ]
                .spacing(4)
                .align_y(alignment::Vertical::Center)
                .into()
            } else {
                text(title).size(12).into()
            };
            button(label)
                .padding(Padding::new(6.0))
                .on_press(on_message(TabMessage::Selected { group, index: i }))
                .into()
        })
        .collect();

    let tab_bar = row(tabs).spacing(2).height(Length::Fixed(TAB_HEIGHT));

    iced::widget::column![tab_bar, active_content]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Tab dock type alias (uses [`tab_dock_simple`]).
pub type TabDock<'a, Message> = Element<'a, Message>;
