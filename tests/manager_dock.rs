use iced_dock::factory::Factory;
use iced_dock::manager::{DockManager, DropZone};
use iced_dock::model::{ContentKey, DockOperation, Layout, TabGroupKind};

#[test]
fn fill_rejects_cross_kind() {
    let factory = Factory;
    let mut layout = Layout::new();
    let doc = factory.insert_document(&mut layout, "d", "D", ContentKey(0));
    let tool = factory.insert_tool(&mut layout, "t", "T", ContentKey(1));
    let dg = factory.create_tab_group(&mut layout, TabGroupKind::Document);
    let tg = factory.create_tab_group(&mut layout, TabGroupKind::Tool);
    factory.add_to_tab_group(&mut layout, dg, doc).unwrap();
    factory.add_to_tab_group(&mut layout, tg, tool).unwrap();

    let dg2 = factory.create_tab_group(&mut layout, TabGroupKind::Document);

    let mgr = DockManager;
    assert!(!mgr.validate(&layout, dg, doc, tg, DockOperation::Fill));
    assert!(mgr.validate(&layout, dg, doc, dg2, DockOperation::Fill));
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
