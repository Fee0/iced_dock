use iced_dock::factory::Factory;
use iced_dock::model::{Axis, ContentKey, DockOperation, Layout, NodeKind};
use iced_dock::TabGroupKind;

#[test]
fn split_merges_into_same_axis_parent() {
    let factory = Factory;
    let mut layout = Layout::new();

    let a = factory.insert_document(&mut layout, "a", "A", ContentKey(0));
    let b = factory.insert_document(&mut layout, "b", "B", ContentKey(1));
    let g = factory.create_tab_group(&mut layout, TabGroupKind::Document);
    factory.add_to_tab_group(&mut layout, g, a).unwrap();
    factory.add_to_tab_group(&mut layout, g, b).unwrap();

    let prop = factory.create_proportional(&mut layout, Axis::Horizontal, vec![g]);
    layout.set_root_child(Some(prop));

    factory
        .split(&mut layout, a, g, DockOperation::Right)
        .unwrap();

    let root = layout.root_child().unwrap();
    let NodeKind::Proportional(pg) = layout.kind(root).unwrap() else {
        panic!("expected proportional root");
    };
    assert_eq!(pg.children.len(), 2);
    assert_eq!(pg.axis, Axis::Horizontal);
}
