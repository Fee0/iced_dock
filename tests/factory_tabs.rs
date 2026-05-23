use iced_dock::factory::Factory;
use iced_dock::model::{ContentKey, Layout, NodeKind};

#[test]
fn fill_and_close_collapses_empty_pane() {
    let factory = Factory;
    let mut layout = Layout::new();

    let a = factory.insert_panel(&mut layout, "a", "A", ContentKey(0));
    let b = factory.insert_panel(&mut layout, "b", "B", ContentKey(1));
    let p1 = factory.create_pane(&mut layout);
    let p2 = factory.create_pane(&mut layout);
    factory.add_panel_to_pane(&mut layout, p1, a).unwrap();
    factory.add_panel_to_pane(&mut layout, p2, b).unwrap();

    factory.dock_fill(&mut layout, a, p2).unwrap();
    factory.close(&mut layout, b).unwrap();

    let NodeKind::Pane(p) = layout.kind(p2).unwrap() else {
        panic!("pane remains");
    };
    assert_eq!(p.tabs.len(), 1);
    assert_eq!(p.active, Some(a));
}
