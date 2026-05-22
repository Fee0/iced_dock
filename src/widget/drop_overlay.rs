//! Drop zone highlights during drag.

use iced::widget::{container, stack};
use iced::{Color, Element, Length, Rectangle, Theme};

use crate::manager::{DragSession, DropZone};
use crate::model::{Layout, NodeId};

/// Overlay five drop zones on `content`.
pub fn drop_overlay<'a, Message: Clone + 'static>(
    content: Element<'a, Message>,
    session: &DragSession,
    layout: &Layout,
    target: NodeId,
    on_drop: impl Fn(Option<NodeId>, DropZone, f32, f32) -> Message + 'a,
) -> Element<'a, Message> {
    let _ = (session, layout, on_drop);
    let highlight = |zone: DropZone| -> Element<'a, Message> {
        let opacity = zone_opacity(zone);
        container(iced::widget::Space::new().width(Length::Fill).height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |theme: &Theme| {
                let c = theme.palette().primary;
                container::Style {
                    background: Some(iced::Background::Color(Color::from_rgba8(
                        (c.r * 255.0) as u8,
                        (c.g * 255.0) as u8,
                        (c.b * 255.0) as u8,
                        opacity,
                    ))),
                    ..container::Style::default()
                }
            })
            .into()
    };

    let base = content;
    let overlay = stack![
        highlight(DropZone::Left),
        highlight(DropZone::Right),
        highlight(DropZone::Top),
        highlight(DropZone::Bottom),
        highlight(DropZone::Center),
    ]
    .width(Length::Fill)
    .height(Length::Fill);

    let _ = target;
    stack![base, overlay].into()
}

fn zone_opacity(zone: DropZone) -> f32 {
    match zone {
        DropZone::Center => 0.15,
        _ => 0.25,
    }
}

/// Register node bounds (placeholder for layout pass integration).
pub fn record_bounds(
    registry: &std::cell::RefCell<Vec<(NodeId, (f32, f32, f32, f32))>>,
    id: NodeId,
    bounds: Rectangle,
) {
    let mut reg = registry.borrow_mut();
    if let Some(entry) = reg.iter_mut().find(|(nid, _)| *nid == id) {
        entry.1 = (bounds.x, bounds.y, bounds.width, bounds.height);
    } else {
        reg.push((id, (bounds.x, bounds.y, bounds.width, bounds.height)));
    }
}
