use iced_dock::factory::{default_ide_layout, Factory};
use iced_dock::model::{ContentKey, Layout, NodeKind, TabGroupKind};

#[test]
fn default_layout_has_documents_and_tools() {
    let mut layout = Layout::new();
    let factory = Factory::new();
    default_ide_layout(
        &factory,
        &mut layout,
        vec![("Doc A".into(), ContentKey(0)), ("Doc B".into(), ContentKey(1))],
        vec![("Tools".into(), ContentKey(10))],
    )
    .unwrap();

    let root_child = layout.root_child().unwrap();
    let NodeKind::Proportional(prop) = layout.kind(root_child).unwrap() else {
        panic!("expected proportional root child");
    };
    assert_eq!(prop.children.len(), 2);
}

#[test]
fn dock_fill_moves_tab_between_groups() {
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
    let tool_group = prop.children[1];

    let doc_leaf = {
        let NodeKind::TabGroup(g) = layout.kind(doc_group).unwrap() else {
            panic!();
        };
        g.children[0]
    };

    factory.dock_fill(&mut layout, doc_leaf, tool_group).unwrap_err();
    // kind mismatch document -> tool group

    let tool_leaf = {
        let NodeKind::TabGroup(g) = layout.kind(tool_group).unwrap() else {
            panic!();
        };
        g.children[0]
    };

    let doc_group2 = factory.create_tab_group(&mut layout, TabGroupKind::Document, None).unwrap();
    factory.add_to_tab_group(&mut layout, doc_group2, tool_leaf).unwrap_err();
}

#[test]
fn close_tab_collapses_empty_group() {
    let mut layout = Layout::new();
    let factory = Factory::new();
    default_ide_layout(
        &factory,
        &mut layout,
        vec![("Only".into(), ContentKey(0))],
        vec![("Tool".into(), ContentKey(10))],
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
        g.children[0]
    };
    factory.close(&mut layout, leaf).unwrap();
    assert!(layout.get(doc_group).is_none() || {
        matches!(
            layout.kind(doc_group),
            Some(NodeKind::TabGroup(g)) if g.children.is_empty()
        )
    });
}
