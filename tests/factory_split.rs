use iced_dock::factory::{default_ide_layout, Factory};
use iced_dock::model::{ContentKey, DockOperation, Layout, NodeKind};

#[test]
fn split_right_adds_proportional_parent() {
    let mut layout = Layout::new();
    let factory = Factory::new();
    default_ide_layout(
        &factory,
        &mut layout,
        vec![("D1".into(), ContentKey(0))],
        vec![("T1".into(), ContentKey(10))],
    )
    .unwrap();

    let root = layout.root_child().unwrap();
    let NodeKind::Proportional(prop) = layout.kind(root).unwrap() else {
        panic!();
    };
    let doc_group = prop.children[0];
    let new_doc = factory
        .insert_document(
            &mut layout,
            iced_dock::DockableMeta::new("d2", "D2", ContentKey(2)),
        )
        .unwrap();

    factory
        .split(
            &mut layout,
            doc_group,
            new_doc,
            DockOperation::Right,
        )
        .unwrap();

    let root2 = layout.root_child().unwrap();
    let NodeKind::Proportional(root_prop) = layout.kind(root2).unwrap() else {
        panic!("root should still be proportional");
    };
    assert!(root_prop.children.len() >= 2);
}

#[test]
fn set_binary_split_ratio_updates_proportions() {
    let mut layout = Layout::new();
    let factory = Factory::new();
    let split = factory
        .create_proportional(
            &mut layout,
            iced_dock::model::Axis::Horizontal,
            vec![],
            None,
        )
        .unwrap();
    let a = factory
        .insert_document(
            &mut layout,
            iced_dock::DockableMeta::new("a", "A", ContentKey(0)),
        )
        .unwrap();
    let b = factory
        .insert_document(
            &mut layout,
            iced_dock::DockableMeta::new("b", "B", ContentKey(1)),
        )
        .unwrap();

    let entry = layout.get_mut(split).unwrap();
    if let NodeKind::Proportional(ref mut g) = entry.kind {
        g.children = vec![a, b];
        g.proportions = vec![0.5, 0.5];
    }
    layout.set_owner(a, Some(split));
    layout.set_owner(b, Some(split));

    factory.set_binary_split_ratio(&mut layout, split, 0.7).unwrap();
    let NodeKind::Proportional(g) = layout.kind(split).unwrap() else {
        panic!();
    };
    assert!((g.proportions[0] - 0.7).abs() < 0.01);
}
