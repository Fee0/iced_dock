use iced_dock::factory::Factory;
use iced_dock::manager::{DockManager, DragSession, DropZone};
use iced_dock::model::{Axis, ContentKey, DockOperation, Layout, NodeKind};

#[test]
fn fill_accepts_panel_into_other_pane() {
    let factory = Factory;
    let mut layout = Layout::new();
    let a = factory.insert_panel(&mut layout, "a", "A", ContentKey(0));
    let b = factory.insert_panel(&mut layout, "b", "B", ContentKey(1));
    let p1 = factory.create_pane(&mut layout);
    let p2 = factory.create_pane(&mut layout);
    factory.add_panel_to_pane(&mut layout, p1, a).unwrap();
    factory.add_panel_to_pane(&mut layout, p2, b).unwrap();

    let mgr = DockManager;
    assert!(mgr.validate(&layout, p1, a, p2, DockOperation::Fill));
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
fn hit_test_pane_prefers_smallest() {
    let factory = Factory;
    let mut layout = Layout::new();
    let p1 = factory.create_pane(&mut layout);
    let p2 = factory.create_pane(&mut layout);
    let targets = [
        (
            p1,
            iced::Rectangle {
                x: 0.0,
                y: 0.0,
                width: 200.0,
                height: 200.0,
            },
        ),
        (
            p2,
            iced::Rectangle {
                x: 50.0,
                y: 50.0,
                width: 50.0,
                height: 50.0,
            },
        ),
    ];
    let hit = DockManager::hit_test_pane(iced::Point::new(75.0, 75.0), &targets);
    assert_eq!(hit.map(|(id, _)| id), Some(p2));
}

#[test]
fn vertical_stack_fill_validates() {
    let factory = Factory;
    let mut layout = Layout::new();
    let a = factory.insert_panel(&mut layout, "top", "Top", ContentKey(0));
    let b = factory.insert_panel(&mut layout, "bot", "Bottom", ContentKey(1));
    let p_top = factory.create_pane(&mut layout);
    let p_bottom = factory.create_pane(&mut layout);
    factory.add_panel_to_pane(&mut layout, p_top, a).unwrap();
    factory.add_panel_to_pane(&mut layout, p_bottom, b).unwrap();
    let col = factory.create_proportional(
        &mut layout,
        Axis::Vertical,
        vec![p_top, p_bottom],
    );
    layout.set_root_child(Some(col));

    let mgr = DockManager;
    assert!(mgr.validate(
        &layout,
        p_bottom,
        b,
        p_top,
        DockOperation::Fill
    ));
    assert!(mgr.validate(
        &layout,
        p_top,
        a,
        p_bottom,
        DockOperation::Fill
    ));
}

#[test]
fn fill_removes_empty_source_pane_from_proportional() {
    let factory = Factory;
    let mut layout = Layout::new();
    let a = factory.insert_panel(&mut layout, "a", "A", ContentKey(0));
    let b = factory.insert_panel(&mut layout, "b", "B", ContentKey(1));
    let p1 = factory.create_pane(&mut layout);
    let p2 = factory.create_pane(&mut layout);
    factory.add_panel_to_pane(&mut layout, p1, a).unwrap();
    factory.add_panel_to_pane(&mut layout, p2, b).unwrap();
    let col = factory.create_proportional(&mut layout, Axis::Vertical, vec![p1, p2]);
    layout.set_root_child(Some(col));

    factory.dock_fill(&mut layout, a, p2).unwrap();

    assert!(layout.get(p1).is_none());
    assert_eq!(layout.root_child(), Some(p2));
    let p2_tabs = match layout.kind(p2) {
        Some(NodeKind::Pane(p)) => p.tabs.clone(),
        _ => panic!("expected pane"),
    };
    assert_eq!(p2_tabs.len(), 2);
    assert!(p2_tabs.contains(&a));
    assert!(p2_tabs.contains(&b));
}

#[test]
fn cross_pane_fill_via_execute() {
    let factory = Factory;
    let mut layout = Layout::new();
    let a = factory.insert_panel(&mut layout, "a", "A", ContentKey(0));
    let b = factory.insert_panel(&mut layout, "b", "B", ContentKey(1));
    let p1 = factory.create_pane(&mut layout);
    let p2 = factory.create_pane(&mut layout);
    factory.add_panel_to_pane(&mut layout, p1, a).unwrap();
    factory.add_panel_to_pane(&mut layout, p2, b).unwrap();
    let col = factory.create_proportional(&mut layout, Axis::Vertical, vec![p1, p2]);
    layout.set_root_child(Some(col));

    let mgr = DockManager;
    let session = DragSession {
        source_pane: p1,
        source_panel: a,
        hover_target: Some(p2),
        operation: Some(DockOperation::Fill),
    };
    mgr.execute(&mut layout, session).unwrap();

    assert!(layout.get(p1).is_none());
    let tabs = match layout.kind(p2) {
        Some(NodeKind::Pane(p)) => p.tabs.clone(),
        _ => panic!("pane"),
    };
    assert_eq!(tabs.len(), 2);
}

#[test]
fn same_pane_edge_validates_and_executes() {
    let factory = Factory;
    let mut layout = Layout::new();
    let a = factory.insert_panel(&mut layout, "a", "A", ContentKey(0));
    let b = factory.insert_panel(&mut layout, "b", "B", ContentKey(1));
    let pane = factory.create_pane(&mut layout);
    factory.add_panel_to_pane(&mut layout, pane, a).unwrap();
    factory.add_panel_to_pane(&mut layout, pane, b).unwrap();
    layout.set_root_child(Some(pane));

    let mgr = DockManager;
    assert!(mgr.validate(
        &layout,
        pane,
        b,
        pane,
        DockOperation::Right
    ));

    let session = DragSession {
        source_pane: pane,
        source_panel: b,
        hover_target: Some(pane),
        operation: Some(DockOperation::Right),
    };
    mgr.execute(&mut layout, session).unwrap();

    let NodeKind::Proportional(pg) = layout.kind(layout.root_child().unwrap()).unwrap() else {
        panic!("split expected");
    };
    assert_eq!(pg.children.len(), 2);
}
