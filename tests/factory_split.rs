use iced_dock::factory::Factory;
use iced_dock::model::{Axis, ContentKey, DockOperation, Layout, NodeKind};

#[test]
fn split_merges_into_same_axis_parent() {
    let factory = Factory;
    let mut layout = Layout::new();

    let a = factory.insert_panel(&mut layout, "a", "A", ContentKey(0));
    let pane_a = factory.create_pane(&mut layout);
    factory.add_panel_to_pane(&mut layout, pane_a, a).unwrap();

    let b = factory.insert_panel(&mut layout, "b", "B", ContentKey(1));
    let pane_b = factory.create_pane(&mut layout);
    factory.add_panel_to_pane(&mut layout, pane_b, b).unwrap();

    let prop = factory.create_proportional(&mut layout, Axis::Horizontal, vec![pane_a]);
    layout.set_root_child(Some(prop));

    factory
        .split(&mut layout, pane_b, pane_a, DockOperation::Right)
        .unwrap();

    let root = layout.root_child().unwrap();
    let NodeKind::Proportional(pg) = layout.kind(root).unwrap() else {
        panic!("expected proportional root");
    };
    assert_eq!(pg.children.len(), 2);
    assert_eq!(pg.axis, Axis::Horizontal);
}

#[test]
fn same_pane_edge_split_multi_panel() {
    let factory = Factory;
    let mut layout = Layout::new();

    let a = factory.insert_panel(&mut layout, "a", "A", ContentKey(0));
    let b = factory.insert_panel(&mut layout, "b", "B", ContentKey(1));
    let pane = factory.create_pane(&mut layout);
    factory.add_panel_to_pane(&mut layout, pane, a).unwrap();
    factory.add_panel_to_pane(&mut layout, pane, b).unwrap();
    layout.set_root_child(Some(pane));

    factory
        .split_same_pane_edge(&mut layout, pane, b, DockOperation::Right)
        .unwrap();

    let root = layout.root_child().unwrap();
    let NodeKind::Proportional(pg) = layout.kind(root).unwrap() else {
        panic!("expected split root");
    };
    assert_eq!(pg.children.len(), 2);
}

#[test]
fn same_pane_edge_split_single_panel() {
    let factory = Factory;
    let mut layout = Layout::new();

    let a = factory.insert_panel(&mut layout, "a", "A", ContentKey(0));
    let pane = factory.create_pane(&mut layout);
    factory.add_panel_to_pane(&mut layout, pane, a).unwrap();
    layout.set_root_child(Some(pane));

    factory
        .split_same_pane_edge(&mut layout, pane, a, DockOperation::Left)
        .unwrap();

    let root = layout.root_child().unwrap();
    let NodeKind::Proportional(pg) = layout.kind(root).unwrap() else {
        panic!("expected split root");
    };
    assert_eq!(pg.children.len(), 2);
    assert_eq!(pg.axis, Axis::Horizontal);
}
