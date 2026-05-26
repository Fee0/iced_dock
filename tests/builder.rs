use iced_dock::unstable::build_tree;
use iced_dock::{
    horizontal, panel, tabs, vertical, DockSession, Error, LayoutTree, PaneTarget,
};

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
fn nested_layout_produces_horizontal_root() {
    let built = build_tree(&nested_layout()).expect("nested layout should compile");
    let root = built.layout.root_child().expect("root child");
    let iced_dock::model::NodeKind::Proportional(pg) =
        built.layout.kind(root).expect("proportional root")
    else {
        panic!("expected proportional root");
    };
    assert_eq!(pg.children.len(), 2);
    assert_eq!(built.index.panels.len(), 7);
}

#[test]
fn duplicate_panel_id_is_rejected() {
    let tree = tabs([
        panel("a", "A", 0u32),
        panel("a", "B", 1u32),
    ]);
    let err = build_tree(&tree).unwrap_err();
    assert_eq!(err, Error::DuplicatePanelId("a".into()));
}

#[test]
fn unknown_active_panel_is_rejected() {
    let tree = tabs([panel("a", "A", 0u32)]).active("missing");
    let err = build_tree(&tree).unwrap_err();
    assert!(matches!(err, Error::UnknownActivePanel { .. }));
}

#[test]
fn mismatched_weights_are_rejected() {
    let tree = horizontal([tabs([panel("a", "A", 0u32)])]).weights([0.5, 0.5]);
    let err = build_tree(&tree).unwrap_err();
    assert!(matches!(err, Error::InvalidWeights { .. }));
}

#[test]
fn simple_split_matches_manual_structure() {
    let tree = horizontal([
        tabs([panel("a", "A", 0u32)]),
        tabs([panel("b", "B", 1u32)]),
    ]);
    let built = build_tree(&tree).expect("compile");
    let root = built.layout.root_child().unwrap();
    let iced_dock::model::NodeKind::Proportional(pg) = built.layout.kind(root).unwrap() else {
        panic!("expected split root");
    };
    assert_eq!(pg.children.len(), 2);
}

#[test]
fn session_open_focus_close_by_id() {
    let session: DockSession<u32> =
        DockSession::from_tree(tabs([panel("a", "A", 0u32)])).expect("session");
    assert_eq!(session.active_panel().as_deref(), Some("a"));

    session
        .open_panel(PaneTarget::First, panel("b", "B", 1u32))
        .expect("open");
    assert!(session.panel_ids().contains(&"b".into()));
    assert_eq!(session.active_panel().as_deref(), Some("b"));

    session.select_panel("a").expect("select");
    assert_eq!(session.active_panel().as_deref(), Some("a"));

    session.close_panel("a").expect("close");
    assert!(!session.panel_ids().contains(&"a".into()));
}

#[test]
fn session_from_tree_sets_layout_dirty() {
    let session: DockSession<u32> =
        DockSession::from_tree(tabs([panel("a", "A", 0u32)])).expect("session");
    assert!(session.state().borrow().layout_dirty);
    assert!(session.state().borrow().layout.root_child().is_some());
}

#[test]
fn named_pane_target_opens_panel() {
    let tree = tabs([panel("a", "A", 0u32)]).named("editor");
    let session: DockSession<u32> = DockSession::from_tree(tree).expect("session");
    session
        .open_panel(
            PaneTarget::Named("editor".into()),
            panel("b", "B", 1u32),
        )
        .expect("open in named pane");
    assert!(session.panel_ids().contains(&"b".into()));
}

#[test]
fn widget_state_from_tree() {
    let state = iced_dock::DockWidgetState::<u32>::from_tree(tabs([panel(
        "a",
        "A",
        0u32,
    )]))
    .expect("state");
    assert!(state.layout_dirty);
    assert!(state.layout.root_child().is_some());
}
