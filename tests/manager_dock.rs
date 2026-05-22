use iced_dock::factory::{default_ide_layout, Factory};
use iced_dock::manager::{DockManager, DragSession};
use iced_dock::model::{ContentKey, DockOperation, Layout, NodeKind};

#[test]
fn manager_executes_fill_dock() {
    let mut layout = Layout::new();
    let factory = Factory::new();
    default_ide_layout(
        &factory,
        &mut layout,
        vec![("D1".into(), ContentKey(0)), ("D2".into(), ContentKey(1))],
        vec![("T1".into(), ContentKey(10))],
    )
    .unwrap();

    let root = layout.root_child().unwrap();
    let NodeKind::Proportional(prop) = layout.kind(root).unwrap() else {
        panic!();
    };
    let doc_group = prop.children[0];

    let leaf = {
        let NodeKind::TabGroup(g) = layout.kind(doc_group).unwrap() else {
            panic!();
        };
        g.children[1]
    };

    let manager = DockManager::new();
    let session = DragSession {
        source: leaf,
        hover_target: Some(doc_group),
        operation: DockOperation::Fill,
    };
    manager.execute(&factory, &mut layout, &session).unwrap();

    let NodeKind::TabGroup(g) = layout.kind(doc_group).unwrap() else {
        panic!();
    };
    assert_eq!(g.children.len(), 2);
}
