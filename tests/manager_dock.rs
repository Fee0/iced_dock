use iced_dock::factory::Factory;
use iced_dock::manager::{DockManager, DropZone};
use iced_dock::model::{Axis, ContentKey, DockOperation, Layout, NodeKind};

#[test]
fn fill_accepts_document_into_tool_group() {
    let factory = Factory;
    let mut layout = Layout::new();
    let doc = factory.insert_document(&mut layout, "d", "D", ContentKey(0));
    let tool = factory.insert_tool(&mut layout, "t", "T", ContentKey(1));
    let dg = factory.create_tab_group(&mut layout);
    let tg = factory.create_tab_group(&mut layout);
    factory.add_to_tab_group(&mut layout, dg, doc).unwrap();
    factory.add_to_tab_group(&mut layout, tg, tool).unwrap();

    let mgr = DockManager;
    assert!(mgr.validate(&layout, dg, doc, tg, DockOperation::Fill));
}

#[test]
fn hit_test_center_zone() {
    let bounds = iced::Rectangle {
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 100.0,
    };
    let zone = DockManager::hit_test_drop_zone(bounds, iced::Point::new(50.0, 50.0));
    assert_eq!(zone, Some(DropZone::Center));
}

#[test]
fn vertical_stack_fill_validates() {
    let factory = Factory;
    let mut layout = Layout::new();
    let d_top = factory.insert_document(&mut layout, "top", "Top", ContentKey(0));
    let d_bottom = factory.insert_document(&mut layout, "bot", "Bottom", ContentKey(1));
    let g_top = factory.create_tab_group(&mut layout);
    let g_bottom = factory.create_tab_group(&mut layout);
    factory.add_to_tab_group(&mut layout, g_top, d_top).unwrap();
    factory.add_to_tab_group(&mut layout, g_bottom, d_bottom).unwrap();
    let col = factory.create_proportional(
        &mut layout,
        Axis::Vertical,
        vec![g_top, g_bottom],
    );
    layout.set_root_child(Some(col));

    let mgr = DockManager;
    assert!(mgr.validate(
        &layout,
        g_bottom,
        d_bottom,
        g_top,
        DockOperation::Fill
    ));
    assert!(mgr.validate(
        &layout,
        g_top,
        d_top,
        g_bottom,
        DockOperation::Fill
    ));
}

#[test]
fn fill_removes_empty_source_group_from_proportional() {
    let factory = Factory;
    let mut layout = Layout::new();
    let d1 = factory.insert_document(&mut layout, "a", "A", ContentKey(0));
    let d2 = factory.insert_document(&mut layout, "b", "B", ContentKey(1));
    let g1 = factory.create_tab_group(&mut layout);
    let g2 = factory.create_tab_group(&mut layout);
    factory.add_to_tab_group(&mut layout, g1, d1).unwrap();
    factory.add_to_tab_group(&mut layout, g2, d2).unwrap();
    let col = factory.create_proportional(&mut layout, Axis::Vertical, vec![g1, g2]);
    layout.set_root_child(Some(col));

    factory.dock_fill(&mut layout, d1, g2).unwrap();

    assert!(layout.get(g1).is_none());
    assert_eq!(layout.root_child(), Some(g2));
    let g2_children = match layout.kind(g2) {
        Some(NodeKind::TabGroup(g)) => g.children.clone(),
        _ => panic!("expected tab group"),
    };
    assert_eq!(g2_children.len(), 2);
    assert!(g2_children.contains(&d1));
    assert!(g2_children.contains(&d2));
}
