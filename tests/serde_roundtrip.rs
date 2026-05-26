use iced_dock::model::NodeKind;
use iced_dock::unstable::{build_tree, BuiltLayout};
use iced_dock::{horizontal, panel, tabs, vertical, Layout, LayoutTree};

fn nested_layout() -> LayoutTree<u32> {
    horizontal([
        vertical([
            tabs([
                panel("main", "main.rs", 0u32),
                panel("lib", "lib.rs", 1u32),
            ])
            .active("main"),
            tabs([panel("preview", "preview", 2u32)]),
        ])
        .weights([0.55, 0.45]),
        vertical([
            tabs([
                panel("props", "Properties", 10u32),
                panel("output", "Output", 11u32),
            ]),
            tabs([
                panel("explorer", "Explorer", 12u32),
                panel("search", "Search", 13u32),
            ]),
        ])
        .weights([0.5, 0.5]),
    ])
    .weights([0.72, 0.28])
}

#[test]
fn layout_tree_json_roundtrip() {
    let tree = nested_layout();
    let json = serde_json::to_string(&tree).unwrap();
    let back: LayoutTree<u32> = serde_json::from_str(&json).unwrap();
    assert_eq!(tree, back);
}

#[test]
fn layout_runtime_json_roundtrip() {
    let tree = nested_layout();
    let built = build_tree(&tree).expect("compile");
    let json = serde_json::to_string(&built.layout).unwrap();
    let back: Layout<u32> = serde_json::from_str(&json).unwrap();
    assert_eq!(built.layout.nodes.len(), back.nodes.len());
    assert_eq!(built.index.panels.len(), 7);

    let root = back.root_child().expect("root child");
    let NodeKind::Proportional(pg) = back.kind(root).expect("proportional root") else {
        panic!("expected proportional root");
    };
    assert_eq!(pg.children.len(), 2);
}

#[test]
fn built_layout_json_roundtrip() {
    let tree = nested_layout();
    let built = build_tree(&tree).expect("compile");
    let json = serde_json::to_string(&built).unwrap();
    let back: BuiltLayout<u32> = serde_json::from_str(&json).unwrap();
    assert_eq!(built.index.panels.len(), back.index.panels.len());
    for id in built.index.panels.keys() {
        assert!(back.index.panels.contains_key(id));
    }
}

#[test]
fn widget_state_rebuilds_index_after_deserialize() {
    let built = build_tree(&nested_layout()).expect("compile");
    let mut state = iced_dock::DockWidgetState::<u32, iced::Theme>::from_built(built, None);
    state.sync_index();
    assert_eq!(state.index.panels.len(), 7);
}
