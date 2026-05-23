use iced_dock::factory::Factory;
use iced_dock::model::{ContentKey, Layout, NodeKind, TabGroupKind};

#[test]
fn fill_and_close_collapses_empty_group() {
    let factory = Factory;
    let mut layout = Layout::new();

    let a = factory.insert_document(&mut layout, "a", "A", ContentKey(0));
    let b = factory.insert_document(&mut layout, "b", "B", ContentKey(1));
    let g1 = factory.create_tab_group(&mut layout, TabGroupKind::Document);
    let g2 = factory.create_tab_group(&mut layout, TabGroupKind::Document);
    factory.add_to_tab_group(&mut layout, g1, a).unwrap();
    factory.add_to_tab_group(&mut layout, g2, b).unwrap();

    factory.dock_fill(&mut layout, a, g2).unwrap();
    factory.close(&mut layout, b).unwrap();

    let NodeKind::TabGroup(g) = layout.kind(g2).unwrap() else {
        panic!("group remains");
    };
    assert_eq!(g.children.len(), 1);
    assert_eq!(g.active, Some(a));
}
