use iced_dock::builder::build_tree;
use iced_dock::{
    adjacent_pane, handle_dock_message, horizontal, owning_pane, pane_bounds_map, panel, tabs,
    vertical, ContentKey, Direction, DockMessage, DockSession, DockWidgetState, PaneTarget,
    TabMessage,
};

fn nested_layout() -> iced_dock::LayoutTree {
    horizontal([
        vertical([
            tabs([
                panel("main", "main.rs", ContentKey(0)),
                panel("lib", "lib.rs", ContentKey(1)),
            ])
            .active("main"),
            tabs([panel("preview", "preview", ContentKey(2))]),
        ])
        .weights([0.55, 0.45]),
        vertical([
            tabs([
                panel("props", "Properties", ContentKey(10)),
                panel("output", "Output", ContentKey(11)),
            ]),
            tabs([
                panel("explorer", "Explorer", ContentKey(12)),
                panel("search", "Search", ContentKey(13)),
            ]),
        ])
        .weights([0.5, 0.5]),
    ])
    .weights([0.72, 0.28])
}

#[test]
fn from_tree_initializes_focused_pane() {
    let session = DockSession::from_tree(nested_layout()).expect("session");
    assert!(session.focused_pane().is_some());
}

#[test]
fn tab_select_sets_focused_pane() {
    let session = DockSession::from_tree(nested_layout()).expect("session");
    let initial = session.focused_pane().expect("initial focus");

    let built = build_tree(&nested_layout()).expect("build");
    let preview_panel = built.index.panel_node("preview").expect("preview");
    let preview_pane = owning_pane(&built.layout, preview_panel).expect("preview pane");
    assert_ne!(initial, preview_pane);

    session.apply_message(DockMessage::Tab(TabMessage::Select {
        pane: preview_pane,
        panel: preview_panel,
    }));

    assert_eq!(session.focused_pane(), Some(preview_pane));
    assert_eq!(session.active_panel().as_deref(), Some("preview"));
}

#[test]
fn pane_focused_updates_focus_without_layout_dirty() {
    let built = build_tree(&horizontal([
        tabs([panel("a", "A", ContentKey(0))]),
        tabs([panel("b", "B", ContentKey(1))]),
    ]))
    .expect("built");
    let pane_a = iced_dock::first_pane(&built.layout).expect("pane a");
    let pane_b = built
        .layout
        .root_child()
        .and_then(|root| {
            if let iced_dock::NodeKind::Proportional(pg) = built.layout.kind(root)? {
                pg.children.iter().find(|&&id| id != pane_a).copied()
            } else {
                None
            }
        })
        .expect("pane b");

    let mut state = DockWidgetState {
        layout: built.layout,
        drag: None,
        drop_targets: Vec::new(),
        tab_bar_targets: Vec::new(),
        pane_bounds: Vec::new(),
        focused_pane: Some(pane_a),
        focus_dirty: false,
        layout_dirty: false,
    };

    let changed = handle_dock_message(
        &mut state,
        DockMessage::PaneFocused {
            pane: pane_b,
            panel: None,
        },
    );

    assert!(changed);
    assert!(!state.layout_dirty);
    assert!(state.focus_dirty);
    assert_eq!(state.focused_pane, Some(pane_b));
}

#[test]
fn active_panel_uses_focused_pane_in_multi_pane_layout() {
    let session = DockSession::from_tree(nested_layout()).expect("session");
    let built = build_tree(&nested_layout()).expect("built");

    let props_panel = built.index.panel_node("props").expect("props");
    let props_pane = owning_pane(&built.layout, props_panel).expect("props pane");

    session.apply_message(DockMessage::Tab(TabMessage::Select {
        pane: props_pane,
        panel: props_panel,
    }));

    assert_eq!(session.focused_pane(), Some(props_pane));
    assert_eq!(session.active_panel().as_deref(), Some("props"));
}

#[test]
fn focus_pane_api() {
    let session = DockSession::from_tree(nested_layout()).expect("session");
    let built = build_tree(&nested_layout()).expect("built");
    let explorer_panel = built.index.panel_node("explorer").expect("explorer");
    let explorer_pane = owning_pane(&built.layout, explorer_panel).expect("pane");

    session.focus_pane(explorer_pane).expect("focus pane");
    assert_eq!(session.focused_pane(), Some(explorer_pane));
    // Last tab added during compile is active in this pane.
    assert_eq!(session.active_panel().as_deref(), Some("search"));
}

#[test]
fn open_panel_active_targets_focused_pane() {
    let session = DockSession::from_tree(nested_layout()).expect("session");
    let built = build_tree(&nested_layout()).expect("built");
    let output_panel = built.index.panel_node("output").expect("output");
    let output_pane = owning_pane(&built.layout, output_panel).expect("pane");

    session.focus_pane(output_pane).expect("focus output pane");
    session
        .open_panel(
            PaneTarget::Active,
            panel("terminal", "Terminal", ContentKey(99)),
        )
        .expect("open");

    assert_eq!(session.active_panel().as_deref(), Some("terminal"));
    assert_eq!(session.focused_pane(), Some(output_pane));
}

#[test]
fn adjacent_pane_finds_horizontal_neighbor_with_gap() {
    let built = build_tree(&horizontal([
        tabs([panel("a", "A", ContentKey(0))]),
        tabs([panel("b", "B", ContentKey(1))]),
    ]))
    .expect("built");
    let left = iced_dock::first_pane(&built.layout).expect("left");
    let right = built
        .layout
        .root_child()
        .and_then(|root| {
            if let iced_dock::NodeKind::Proportional(pg) = built.layout.kind(root)? {
                pg.children.iter().find(|&&id| id != left).copied()
            } else {
                None
            }
        })
        .expect("right");

    let mut map = std::collections::HashMap::new();
    map.insert(
        left,
        iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(100.0, 100.0)),
    );
    map.insert(
        right,
        iced::Rectangle::new(iced::Point::new(120.0, 0.0), iced::Size::new(100.0, 100.0)),
    );

    assert_eq!(adjacent_pane(left, Direction::Right, &map), Some(right));
    assert_eq!(adjacent_pane(right, Direction::Left, &map), Some(left));
}

#[test]
fn adjacent_pane_finds_horizontal_neighbor() {
    let built = build_tree(&horizontal([
        tabs([panel("a", "A", ContentKey(0))]),
        tabs([panel("b", "B", ContentKey(1))]),
    ]))
    .expect("built");
    let left = iced_dock::first_pane(&built.layout).expect("left");
    let right = built
        .layout
        .root_child()
        .and_then(|root| {
            if let iced_dock::NodeKind::Proportional(pg) = built.layout.kind(root)? {
                pg.children.iter().find(|&&id| id != left).copied()
            } else {
                None
            }
        })
        .expect("right");

    let mut map = std::collections::HashMap::new();
    map.insert(
        left,
        iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(100.0, 100.0)),
    );
    map.insert(
        right,
        iced::Rectangle::new(iced::Point::new(100.0, 0.0), iced::Size::new(100.0, 100.0)),
    );

    assert_eq!(adjacent_pane(left, Direction::Right, &map), Some(right));
    assert_eq!(adjacent_pane(right, Direction::Left, &map), Some(left));
}

#[test]
fn pane_bounds_map_from_collected_vec() {
    let built = build_tree(&horizontal([
        tabs([panel("a", "A", ContentKey(0))]),
        tabs([panel("b", "B", ContentKey(1))]),
    ]))
    .expect("built");
    let a = iced_dock::first_pane(&built.layout).expect("a");
    let b = built
        .layout
        .root_child()
        .and_then(|root| {
            if let iced_dock::NodeKind::Proportional(pg) = built.layout.kind(root)? {
                pg.children.iter().find(|&&id| id != a).copied()
            } else {
                None
            }
        })
        .expect("b");

    let bounds = vec![
        (
            a,
            iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(10.0, 10.0)),
        ),
        (
            b,
            iced::Rectangle::new(iced::Point::new(20.0, 0.0), iced::Size::new(10.0, 10.0)),
        ),
    ];
    let map = pane_bounds_map(&bounds);
    assert_eq!(map.len(), 2);
    assert!(map.contains_key(&a));
}
