use iced_dock::model::{Axis, ContentKey, DockOperation, Layout, NodeId, NodeKind};
use iced_dock::unstable::Factory;

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

fn three_pane_group(factory: &Factory, layout: &mut Layout) -> NodeId {
    let panes: Vec<_> = (0..3).map(|_| factory.create_pane(layout)).collect();
    let group =
        factory.create_proportional(layout, Axis::Horizontal, vec![panes[0], panes[1], panes[2]]);
    factory
        .set_proportions(layout, group, vec![2.0, 3.0, 5.0])
        .unwrap();
    group
}

fn four_pane_group(factory: &Factory, layout: &mut Layout) -> NodeId {
    let panes: Vec<_> = (0..4).map(|_| factory.create_pane(layout)).collect();
    let group = factory.create_proportional(
        layout,
        Axis::Horizontal,
        vec![panes[0], panes[1], panes[2], panes[3]],
    );
    factory
        .set_proportions(layout, group, vec![1.0, 2.0, 3.0, 4.0])
        .unwrap();
    group
}

fn approx_eq(a: f32, b: f32) {
    assert!((a - b).abs() < 1e-5, "expected {b}, got {a}");
}

#[test]
fn adjust_splitter_middle_only_moves_adjacent_pair() {
    let factory = Factory;
    let mut layout = Layout::new();
    let group = three_pane_group(&factory, &mut layout);

    factory
        .adjust_splitter(&mut layout, group, 1, 0.25)
        .unwrap();

    let NodeKind::Proportional(pg) = layout.kind(group).unwrap() else {
        panic!("expected proportional group");
    };
    approx_eq(pg.proportions[0], 2.0 / 10.0);
    approx_eq(pg.proportions[1], 2.0 / 10.0);
    approx_eq(pg.proportions[2], 6.0 / 10.0);
    approx_eq(pg.proportions[1] + pg.proportions[2], 0.8);
}

#[test]
fn adjust_splitter_four_pane_keeps_outer_panes_fixed() {
    let factory = Factory;
    let mut layout = Layout::new();
    let group = four_pane_group(&factory, &mut layout);

    factory.adjust_splitter(&mut layout, group, 1, 0.5).unwrap();

    let NodeKind::Proportional(pg) = layout.kind(group).unwrap() else {
        panic!("expected proportional group");
    };
    approx_eq(pg.proportions[0], 1.0 / 10.0);
    approx_eq(pg.proportions[3], 4.0 / 10.0);
    approx_eq(pg.proportions[1], 2.5 / 10.0);
    approx_eq(pg.proportions[2], 2.5 / 10.0);
}

#[test]
fn adjust_splitter_first_divider_only_moves_first_pair() {
    let factory = Factory;
    let mut layout = Layout::new();
    let group = three_pane_group(&factory, &mut layout);

    factory.adjust_splitter(&mut layout, group, 0, 0.6).unwrap();

    let NodeKind::Proportional(pg) = layout.kind(group).unwrap() else {
        panic!("expected proportional group");
    };
    approx_eq(pg.proportions[2], 5.0 / 10.0);
    approx_eq(pg.proportions[0], 3.0 / 10.0);
    approx_eq(pg.proportions[1], 2.0 / 10.0);
}
