//! Recursive layout tree → iced `Element`.

use std::rc::Rc;

use iced::widget::{container, text};
use iced::{Element, Length};

use crate::manager::{hit_test_drop_zone, DragSession, DropZone};
use crate::model::{ContentKey, Layout, NodeId, NodeKind};
use crate::view::proportional::fold_proportional;
use crate::widget::dock_surface::DockMessage;
use crate::widget::drop_overlay::drop_overlay;
use crate::widget::tab_dock::{tab_dock_simple, TabMessage};

pub fn build_layout<'a, Message: Clone + 'static>(
    layout: &'a Layout,
    drag: Option<&DragSession>,
    on_dock_message: Rc<dyn Fn(DockMessage) -> Message>,
    content: &'static dyn Fn(ContentKey) -> Element<'static, Message>,
    node: NodeId,
) -> Element<'a, Message> {
    build_node(layout, drag, on_dock_message, content, node)
}

fn build_node<'a, Message: Clone + 'static>(
    layout: &'a Layout,
    drag: Option<&DragSession>,
    on_dock_message: Rc<dyn Fn(DockMessage) -> Message>,
    content: &'static dyn Fn(ContentKey) -> Element<'static, Message>,
    node: NodeId,
) -> Element<'a, Message> {
    let Some(kind) = layout.kind(node).cloned() else {
        return text("missing").into();
    };

    match kind {
        NodeKind::Root(r) => {
            if let Some(child) = r.child {
                build_node(layout, drag, on_dock_message, content, child)
            } else {
                text("empty layout").into()
            }
        }
        NodeKind::Proportional(group) => {
            let group_id = node;
            let on_msg = on_dock_message.clone();
            fold_proportional(
                &group,
                |child_id| build_node(layout, drag, on_dock_message.clone(), content, child_id),
                move |ratio| {
                    on_msg(DockMessage::SplitDrag {
                        group: group_id,
                        split_at: ratio,
                    })
                },
            )
        }
        NodeKind::TabGroup(_) => build_tab_group(
            layout,
            drag,
            on_dock_message,
            content,
            node,
        ),
        NodeKind::Document(meta) | NodeKind::Tool(meta) => content(meta.content),
    }
}

fn build_tab_group<'a, Message: Clone + 'static>(
    layout: &'a Layout,
    drag: Option<&DragSession>,
    on_dock_message: Rc<dyn Fn(DockMessage) -> Message>,
    content: &'static dyn Fn(ContentKey) -> Element<'static, Message>,
    group: NodeId,
) -> Element<'a, Message> {
    let active_leaf = layout.kind(group).and_then(|k| {
        if let NodeKind::TabGroup(g) = k {
            g.active.or_else(|| g.children.first().copied())
        } else {
            None
        }
    });

    let active_content = if let Some(leaf) = active_leaf {
        build_node(layout, drag, on_dock_message.clone(), content, leaf)
    } else {
        text("no tabs").into()
    };

    let on_msg = on_dock_message.clone();
    let tab_ui = tab_dock_simple(group, layout, active_content, move |msg| {
        on_msg(DockMessage::Tab(msg))
    });

    if let Some(session) = drag {
        container(
            drop_overlay(
                tab_ui,
                session,
                layout,
                group,
                {
                    let on_msg = on_dock_message.clone();
                    move |target, zone, x, y| {
                        on_msg(DockMessage::Tab(TabMessage::DragEnded {
                        target,
                        zone,
                        x,
                        y,
                    }))
                    }
                },
            ),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    } else {
        tab_ui
    }
}

/// Resolve drag end target from global pointer (used by dock surface).
pub fn resolve_drag_target(
    registry: &[(NodeId, (f32, f32, f32, f32))],
    x: f32,
    y: f32,
) -> Option<(NodeId, DropZone)> {
    for &(id, bounds) in registry.iter().rev() {
        let (bx, by, bw, bh) = bounds;
        if x >= bx && x <= bx + bw && y >= by && y <= by + bh {
            let zone = hit_test_drop_zone(bounds, x, y);
            return Some((id, zone));
        }
    }
    None
}
