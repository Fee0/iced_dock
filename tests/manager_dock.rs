use iced_dock::unstable::{DockManager, DragSession, DropZone, Factory, TabBarTarget};
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
fn hit_test_respects_custom_edge_fraction() {
    let bounds = iced::Rectangle {
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 100.0,
    };
    let edge = 0.2;
    assert_eq!(
        DockManager::hit_test_drop_zone(bounds, iced::Point::new(15.0, 50.0), edge),
        Some(DropZone::Left)
    );
    assert_eq!(
        DockManager::hit_test_drop_zone(bounds, iced::Point::new(15.0, 50.0), 0.1),
        Some(DropZone::Center)
    );
}

#[test]
fn hit_test_center_zone() {
    let bounds = iced::Rectangle {
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 100.0,
    };
    let zone = DockManager::hit_test_drop_zone(bounds, iced::Point::new(50.0, 50.0), 0.2);
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
    let hit = DockManager::hit_test_pane(iced::Point::new(75.0, 75.0), &targets, 0.2);
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
    let col = factory.create_proportional(&mut layout, Axis::Vertical, vec![p_top, p_bottom]);
    layout.set_root_child(Some(col));

    let mgr = DockManager;
    assert!(mgr.validate(&layout, p_bottom, b, p_top, DockOperation::Fill));
    assert!(mgr.validate(&layout, p_top, a, p_bottom, DockOperation::Fill));
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
        tab_insert: None,
        drop_edge_fraction: 0.2,
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
    assert!(mgr.validate(&layout, pane, b, pane, DockOperation::Right));

    let session = DragSession {
        source_pane: pane,
        source_panel: b,
        hover_target: Some(pane),
        operation: Some(DockOperation::Right),
        tab_insert: None,
        drop_edge_fraction: 0.2,
    };
    mgr.execute(&mut layout, session).unwrap();

    let NodeKind::Proportional(pg) = layout.kind(layout.root_child().unwrap()).unwrap() else {
        panic!("split expected");
    };
    assert_eq!(pg.children.len(), 2);
}

#[test]
fn cross_pane_edge_split_multi_panel() {
    let factory = Factory;
    let mut layout = Layout::new();
    let a = factory.insert_panel(&mut layout, "a", "A", ContentKey(0));
    let b = factory.insert_panel(&mut layout, "b", "B", ContentKey(1));
    let c = factory.insert_panel(&mut layout, "c", "C", ContentKey(2));
    let p1 = factory.create_pane(&mut layout);
    let p2 = factory.create_pane(&mut layout);
    factory.add_panel_to_pane(&mut layout, p1, a).unwrap();
    factory.add_panel_to_pane(&mut layout, p1, b).unwrap();
    factory.add_panel_to_pane(&mut layout, p2, c).unwrap();
    let row = factory.create_proportional(&mut layout, Axis::Horizontal, vec![p1, p2]);
    layout.set_root_child(Some(row));

    let mgr = DockManager;
    let session = DragSession {
        source_pane: p1,
        source_panel: b,
        hover_target: Some(p2),
        operation: Some(DockOperation::Right),
        tab_insert: None,
        drop_edge_fraction: 0.2,
    };
    mgr.execute(&mut layout, session).unwrap();

    let p1_tabs = match layout.kind(p1) {
        Some(NodeKind::Pane(p)) => p.tabs.clone(),
        _ => panic!("expected pane"),
    };
    assert_eq!(p1_tabs, vec![a]);

    let b_pane = layout.get(b).and_then(|e| e.owner).expect("b owner");
    assert_ne!(b_pane, p1);

    let p2_tabs = match layout.kind(p2) {
        Some(NodeKind::Pane(p)) => p.tabs.clone(),
        _ => panic!("expected pane"),
    };
    assert_eq!(p2_tabs, vec![c]);

    let root = layout.root_child().unwrap();
    let NodeKind::Proportional(pg) = layout.kind(root).unwrap() else {
        panic!("expected horizontal split root");
    };
    assert_eq!(pg.children.len(), 3);
    assert!(pg.children.contains(&p1));
    assert!(pg.children.contains(&p2));
    assert!(pg.children.contains(&b_pane));
}

#[test]
fn hit_test_tab_insert_picks_index_from_boundaries() {
    let factory = Factory;
    let pane = factory.create_pane(&mut Layout::new());
    let bounds = iced::Rectangle {
        x: 0.0,
        y: 0.0,
        width: 300.0,
        height: 30.0,
    };
    let targets = [TabBarTarget {
        pane,
        bounds,
        insert_x: vec![0.0, 100.0, 200.0, 300.0],
        scroll_offset: 0.0,
    }];

    assert_eq!(
        DockManager::hit_test_tab_insert(iced::Point::new(40.0, 10.0), &targets),
        Some((pane, 0))
    );
    assert_eq!(
        DockManager::hit_test_tab_insert(iced::Point::new(140.0, 10.0), &targets),
        Some((pane, 1))
    );
    assert_eq!(
        DockManager::hit_test_tab_insert(iced::Point::new(240.0, 10.0), &targets),
        Some((pane, 2))
    );
    assert_eq!(
        DockManager::hit_test_tab_insert(iced::Point::new(290.0, 10.0), &targets),
        Some((pane, 3))
    );
}

#[test]
fn same_pane_tab_reorder_via_execute_tab_insert() {
    let factory = Factory;
    let mut layout = Layout::new();
    let a = factory.insert_panel(&mut layout, "a", "A", ContentKey(0));
    let b = factory.insert_panel(&mut layout, "b", "B", ContentKey(1));
    let c = factory.insert_panel(&mut layout, "c", "C", ContentKey(2));
    let pane = factory.create_pane(&mut layout);
    factory.add_panel_to_pane(&mut layout, pane, a).unwrap();
    factory.add_panel_to_pane(&mut layout, pane, b).unwrap();
    factory.add_panel_to_pane(&mut layout, pane, c).unwrap();
    layout.set_root_child(Some(pane));

    let session = DragSession {
        source_pane: pane,
        source_panel: c,
        hover_target: None,
        operation: None,
        tab_insert: None,
        drop_edge_fraction: 0.2,
    };
    DockManager
        .execute_tab_insert(&mut layout, session, pane, 0)
        .unwrap();

    let tabs = match layout.kind(pane) {
        Some(NodeKind::Pane(p)) => p.tabs.clone(),
        _ => panic!("expected pane"),
    };
    assert_eq!(tabs, vec![c, a, b]);
}

#[test]
fn cross_pane_tab_insert_at_index_zero() {
    let factory = Factory;
    let mut layout = Layout::new();
    let a = factory.insert_panel(&mut layout, "a", "A", ContentKey(0));
    let b = factory.insert_panel(&mut layout, "b", "B", ContentKey(1));
    let p1 = factory.create_pane(&mut layout);
    let p2 = factory.create_pane(&mut layout);
    factory.add_panel_to_pane(&mut layout, p1, a).unwrap();
    factory.add_panel_to_pane(&mut layout, p2, b).unwrap();
    let row = factory.create_proportional(&mut layout, Axis::Horizontal, vec![p1, p2]);
    layout.set_root_child(Some(row));

    let session = DragSession {
        source_pane: p1,
        source_panel: a,
        hover_target: None,
        operation: None,
        tab_insert: None,
        drop_edge_fraction: 0.2,
    };
    DockManager
        .execute_tab_insert(&mut layout, session, p2, 0)
        .unwrap();

    assert!(layout.get(p1).is_none());
    let tabs = match layout.kind(p2) {
        Some(NodeKind::Pane(p)) => p.tabs.clone(),
        _ => panic!("expected pane"),
    };
    assert_eq!(tabs, vec![a, b]);
}

#[test]
fn tab_bar_insert_takes_priority_over_content_hover() {
    let factory = Factory;
    let mut layout = Layout::new();
    let pane = factory.create_pane(&mut layout);
    let mut session = DragSession::new(pane, pane, 0.2);
    let drop_targets = [(
        pane,
        iced::Rectangle {
            x: 0.0,
            y: 30.0,
            width: 200.0,
            height: 200.0,
        },
    )];
    let tab_bar_targets = [TabBarTarget {
        pane,
        bounds: iced::Rectangle {
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: 30.0,
        },
        insert_x: vec![0.0, 200.0],
        scroll_offset: 0.0,
    }];

    DockManager::update_drag_hover_full(
        &mut session,
        iced::Point::new(50.0, 10.0),
        &drop_targets,
        &tab_bar_targets,
    );

    assert_eq!(session.tab_insert, Some((pane, 0)));
    assert!(session.hover_target.is_none());
    assert!(session.operation.is_none());
}

#[test]
fn hit_test_tab_insert_with_scroll_offset() {
    let factory = Factory;
    let pane = factory.create_pane(&mut Layout::new());
    let bounds = iced::Rectangle {
        x: 50.0,
        y: 0.0,
        width: 300.0,
        height: 30.0,
    };
    let targets = [TabBarTarget {
        pane,
        bounds,
        insert_x: vec![0.0, 100.0, 200.0, 300.0, 400.0],
        scroll_offset: 100.0,
    }];

    // Screen x=80 → layout x=180 → between tabs at 100 and 200
    assert_eq!(
        DockManager::hit_test_tab_insert(iced::Point::new(80.0, 10.0), &targets),
        Some((pane, 2))
    );
}
