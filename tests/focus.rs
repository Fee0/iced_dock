use iced_dock::model::{Axis, NodeId, NodeKind};
use iced_dock::unstable::{build_tree, dispatch_action, owning_pane};
use iced_dock::{
    adjacent_pane, horizontal, pane_bounds_map, panel, tabs, vertical, Direction, DockAction,
    DockSession, DockWidgetState, InitialFocus, PaneTarget, PanelCycle, TabAction,
};

fn nested_layout() -> iced_dock::LayoutTree<u32> {
    horizontal([
        vertical([
            tabs([panel("main", "main.rs", 0u32), panel("lib", "lib.rs", 1u32)]).active("main"),
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

fn two_panes(built: &iced_dock::unstable::BuiltLayout<u32>) -> (NodeId, NodeId) {
    let left = iced_dock::unstable::first_pane(&built.layout).expect("left");
    let right = built
        .layout
        .root_child()
        .and_then(|root| {
            if let NodeKind::Proportional(pg) = built.layout.kind(root)? {
                pg.children.iter().find(|&&id| id != left).copied()
            } else {
                None
            }
        })
        .expect("right");
    (left, right)
}

fn set_horizontal_bounds(session: &DockSession<u32>, left: NodeId, right: NodeId) {
    session.state().borrow_mut().pane_bounds = vec![
        (
            left,
            iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(100.0, 100.0)),
        ),
        (
            right,
            iced::Rectangle::new(iced::Point::new(100.0, 0.0), iced::Size::new(100.0, 100.0)),
        ),
    ];
}

fn root_split(session: &DockSession<u32>) -> (Axis, Vec<NodeId>) {
    let state = session.state();
    let state = state.borrow();
    let root = state.layout.root_child().expect("root child");
    let NodeKind::Proportional(pg) = state.layout.kind(root).expect("root split") else {
        panic!("expected proportional root");
    };
    (pg.axis, pg.children.clone())
}

#[test]
fn from_tree_initializes_focused_pane() {
    let session: DockSession<u32> = DockSession::from_tree(nested_layout()).expect("session");
    assert!(session.focused_pane().is_some());
}

#[test]
fn tab_select_sets_focused_pane() {
    let session: DockSession<u32> = DockSession::from_tree(nested_layout()).expect("session");
    let initial = session.focused_pane().expect("initial focus");

    let built = build_tree(&nested_layout()).expect("build");
    let preview_panel = built.index.panel_node("preview").expect("preview");
    let preview_pane = owning_pane(&built.layout, preview_panel).expect("preview pane");
    assert_ne!(initial, preview_pane);

    session.dispatch(DockAction::Tab(TabAction::Select {
        pane: preview_pane,
        panel: preview_panel,
    }));

    assert_eq!(session.focused_pane(), Some(preview_pane));
    assert_eq!(session.active_panel().as_deref(), Some("preview"));
}

#[test]
fn pane_focused_updates_focus_without_layout_dirty() {
    let built = build_tree(&horizontal([
        tabs([panel("a", "A", 0u32)]),
        tabs([panel("b", "B", 1u32)]),
    ]))
    .expect("built");
    let pane_a = iced_dock::unstable::first_pane(&built.layout).expect("pane a");
    let pane_b = built
        .layout
        .root_child()
        .and_then(|root| {
            if let iced_dock::model::NodeKind::Proportional(pg) = built.layout.kind(root)? {
                pg.children.iter().find(|&&id| id != pane_a).copied()
            } else {
                None
            }
        })
        .expect("pane b");

    let mut state = DockWidgetState {
        layout: built.layout,
        index: built.index,
        drag: None,
        drop_targets: Vec::new(),
        tab_bar_targets: Vec::new(),
        pane_bounds: Vec::new(),
        focused_pane: Some(pane_a),
        focus_dirty: false,
        layout_dirty: false,
    };

    let changed = dispatch_action(
        &mut state,
        DockAction::PaneFocused {
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
    let session: DockSession<u32> = DockSession::from_tree(nested_layout()).expect("session");
    let built = build_tree(&nested_layout()).expect("built");

    let props_panel = built.index.panel_node("props").expect("props");
    let props_pane = owning_pane(&built.layout, props_panel).expect("props pane");

    session.dispatch(DockAction::Tab(TabAction::Select {
        pane: props_pane,
        panel: props_panel,
    }));

    assert_eq!(session.focused_pane(), Some(props_pane));
    assert_eq!(session.active_panel().as_deref(), Some("props"));
}

#[test]
fn focus_pane_api() {
    let session: DockSession<u32> = DockSession::from_tree(nested_layout()).expect("session");
    let built = build_tree(&nested_layout()).expect("built");
    let explorer_panel = built.index.panel_node("explorer").expect("explorer");
    let explorer_pane = owning_pane(&built.layout, explorer_panel).expect("pane");

    session.focus_pane(explorer_pane).expect("focus pane");
    assert_eq!(session.focused_pane(), Some(explorer_pane));
    assert_eq!(session.active_panel().as_deref(), Some("search"));
}

#[test]
fn open_panel_active_targets_focused_pane() {
    let session: DockSession<u32> = DockSession::from_tree(nested_layout()).expect("session");
    let built = build_tree(&nested_layout()).expect("built");
    let output_panel = built.index.panel_node("output").expect("output");
    let output_pane = owning_pane(&built.layout, output_panel).expect("pane");

    session.focus_pane(output_pane).expect("focus output pane");
    session
        .open_panel(PaneTarget::Active, panel("terminal", "Terminal", 99u32))
        .expect("open");

    assert_eq!(session.active_panel().as_deref(), Some("terminal"));
    assert_eq!(session.focused_pane(), Some(output_pane));
}

#[test]
fn adjacent_pane_finds_horizontal_neighbor_with_gap() {
    let built = build_tree(&horizontal([
        tabs([panel("a", "A", 0u32)]),
        tabs([panel("b", "B", 1u32)]),
    ]))
    .expect("built");
    let left = iced_dock::unstable::first_pane(&built.layout).expect("left");
    let right = built
        .layout
        .root_child()
        .and_then(|root| {
            if let iced_dock::model::NodeKind::Proportional(pg) = built.layout.kind(root)? {
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
        tabs([panel("a", "A", 0u32)]),
        tabs([panel("b", "B", 1u32)]),
    ]))
    .expect("built");
    let left = iced_dock::unstable::first_pane(&built.layout).expect("left");
    let right = built
        .layout
        .root_child()
        .and_then(|root| {
            if let iced_dock::model::NodeKind::Proportional(pg) = built.layout.kind(root)? {
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
        tabs([panel("a", "A", 0u32)]),
        tabs([panel("b", "B", 1u32)]),
    ]))
    .expect("built");
    let a = iced_dock::unstable::first_pane(&built.layout).expect("a");
    let b = built
        .layout
        .root_child()
        .and_then(|root| {
            if let iced_dock::model::NodeKind::Proportional(pg) = built.layout.kind(root)? {
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

#[test]
fn select_panel_by_string_id() {
    let session: DockSession<u32> = DockSession::from_tree(nested_layout()).expect("session");
    session.select_panel("preview").expect("select");
    assert_eq!(session.active_panel().as_deref(), Some("preview"));
}

#[test]
fn active_panel_in_pane_non_focused() {
    let session: DockSession<u32> = DockSession::from_tree(nested_layout()).expect("session");
    let built = build_tree(&nested_layout()).expect("built");
    let output_panel = built.index.panel_node("output").expect("output");
    let output_pane = owning_pane(&built.layout, output_panel).expect("pane");

    session.select_panel("main").expect("focus main pane");
    assert_eq!(
        session.active_panel_in_pane(output_pane).as_deref(),
        Some("output")
    );
    assert_eq!(session.active_panel().as_deref(), Some("main"));
}

#[test]
fn pane_focused_with_panel_activates_tab() {
    let built = build_tree(&horizontal([tabs([
        panel("a", "A", 0u32),
        panel("b", "B", 1u32),
    ])
    .active("a")]))
    .expect("built");
    let pane = iced_dock::unstable::first_pane(&built.layout).expect("pane");
    let panel_b = built.index.panel_node("b").expect("b");

    let mut state = DockWidgetState::<u32>::from_built(built, Some(pane));
    state.layout_dirty = false;

    let changed = dispatch_action(
        &mut state,
        DockAction::PaneFocused {
            pane,
            panel: Some(panel_b),
        },
    );

    assert!(changed);
    assert!(state.layout_dirty);
    assert_eq!(state.focused_pane, Some(pane));
}

#[test]
fn focus_adjacent_moves_focus() {
    let built = build_tree(&horizontal([
        tabs([panel("a", "A", 0u32)]),
        tabs([panel("b", "B", 1u32)]),
    ]))
    .expect("built");
    let (left, right) = two_panes(&built);

    let session: DockSession<u32> = DockSession::from_built(built, Some(left));
    set_horizontal_bounds(&session, left, right);

    assert!(session.focus_adjacent(Direction::Right));
    assert_eq!(session.focused_pane(), Some(right));
}

#[test]
fn move_active_panel_adjacent_moves_tab_and_focus() {
    let built = build_tree(&horizontal([
        tabs([panel("a", "A", 0u32), panel("c", "C", 2u32)]).active("a"),
        tabs([panel("b", "B", 1u32)]),
    ]))
    .expect("built");
    let (left, right) = two_panes(&built);
    let session: DockSession<u32> = DockSession::from_built(built, Some(left));
    set_horizontal_bounds(&session, left, right);

    assert!(session.move_active_panel_adjacent(Direction::Right));
    assert_eq!(session.focused_pane(), Some(right));
    assert_eq!(session.active_panel().as_deref(), Some("a"));
    assert_eq!(session.active_panel_in_pane(left).as_deref(), Some("c"));
    assert_eq!(session.pane_for_panel("a"), Some(right));
}

#[test]
fn move_active_panel_adjacent_collapses_empty_source_pane() {
    let built = build_tree(&horizontal([
        tabs([panel("a", "A", 0u32)]),
        tabs([panel("b", "B", 1u32)]),
    ]))
    .expect("built");
    let (left, right) = two_panes(&built);
    let session: DockSession<u32> = DockSession::from_built(built, Some(left));
    set_horizontal_bounds(&session, left, right);

    assert!(session.move_active_panel_adjacent(Direction::Right));
    assert_eq!(session.focused_pane(), Some(right));
    assert_eq!(session.active_panel().as_deref(), Some("a"));
    assert_eq!(session.pane_for_panel("a"), Some(right));
    assert!(session.state().borrow().layout.get(left).is_none());
}

#[test]
fn move_active_panel_adjacent_without_neighbor_is_noop() {
    let built = build_tree(&horizontal([
        tabs([panel("a", "A", 0u32)]),
        tabs([panel("b", "B", 1u32)]),
    ]))
    .expect("built");
    let (left, right) = two_panes(&built);
    let session: DockSession<u32> = DockSession::from_built(built, Some(left));
    set_horizontal_bounds(&session, left, right);

    assert!(!session.move_active_panel_adjacent(Direction::Left));
    assert_eq!(session.focused_pane(), Some(left));
    assert_eq!(session.active_panel().as_deref(), Some("a"));
    assert_eq!(session.pane_for_panel("a"), Some(left));
}

#[test]
fn move_active_panel_adjacent_rejects_incompatible_groups() {
    let built = build_tree(&horizontal([
        tabs([panel("a", "A", 0u32).group("documents")])
            .active("a")
            .group("documents"),
        tabs([panel("b", "B", 1u32).group("tools")]).group("tools"),
    ]))
    .expect("built");
    let (left, right) = two_panes(&built);
    let session: DockSession<u32> = DockSession::from_built(built, Some(left));
    set_horizontal_bounds(&session, left, right);

    assert!(!session.move_active_panel_adjacent(Direction::Right));
    assert_eq!(session.focused_pane(), Some(left));
    assert_eq!(session.active_panel().as_deref(), Some("a"));
    assert_eq!(session.pane_for_panel("a"), Some(left));
}

#[test]
fn split_active_panel_right_moves_tab_into_new_pane() {
    let built = build_tree(&tabs([panel("a", "A", 0u32), panel("c", "C", 2u32)]).active("a"))
        .expect("built");
    let source = iced_dock::unstable::first_pane(&built.layout).expect("source");
    let session: DockSession<u32> = DockSession::from_built(built, Some(source));

    assert!(session.split_active_panel(Direction::Right));
    let target = session.pane_for_panel("a").expect("target");
    assert_ne!(source, target);
    assert_eq!(session.focused_pane(), Some(target));
    assert_eq!(session.active_panel().as_deref(), Some("a"));
    assert_eq!(session.active_panel_in_pane(source).as_deref(), Some("c"));

    let (axis, children) = root_split(&session);
    assert_eq!(axis, Axis::Horizontal);
    assert_eq!(children, vec![source, target]);
}

#[test]
fn split_active_panel_places_new_pane_in_requested_direction() {
    for (direction, expected_axis, expected_new_index) in [
        (Direction::Left, Axis::Horizontal, 0),
        (Direction::Up, Axis::Vertical, 0),
        (Direction::Down, Axis::Vertical, 1),
    ] {
        let built = build_tree(&tabs([panel("a", "A", 0u32), panel("c", "C", 2u32)]).active("a"))
            .expect("built");
        let source = iced_dock::unstable::first_pane(&built.layout).expect("source");
        let session: DockSession<u32> = DockSession::from_built(built, Some(source));

        assert!(session.split_active_panel(direction));
        let target = session.pane_for_panel("a").expect("target");
        let (axis, children) = root_split(&session);
        assert_eq!(axis, expected_axis);
        assert_eq!(children[expected_new_index], target);
        assert!(children.contains(&source));
    }
}

#[test]
fn split_active_panel_single_tab_is_noop() {
    let built = build_tree(&tabs([panel("a", "A", 0u32)])).expect("built");
    let source = iced_dock::unstable::first_pane(&built.layout).expect("source");
    let session: DockSession<u32> = DockSession::from_built(built, Some(source));

    assert!(!session.split_active_panel(Direction::Right));
    assert_eq!(session.focused_pane(), Some(source));
    assert_eq!(session.pane_for_panel("a"), Some(source));
    assert_eq!(session.state().borrow().layout.root_child(), Some(source));
}

#[test]
fn split_active_panel_without_focus_is_noop() {
    let built = build_tree(&tabs([panel("a", "A", 0u32), panel("c", "C", 2u32)]).active("a"))
        .expect("built");
    let session: DockSession<u32> = DockSession::from_built(built, None);

    assert!(!session.split_active_panel(Direction::Right));
    assert!(session.focused_pane().is_none());
    assert_eq!(session.active_panel(), None);
}

#[test]
fn cycle_panel_wraps() {
    let session: DockSession<u32> = DockSession::from_tree(nested_layout()).expect("session");
    session.select_panel("main").expect("main");
    assert_eq!(session.active_panel().as_deref(), Some("main"));

    session.cycle_panel(PanelCycle::Next).expect("next");
    assert_eq!(session.active_panel().as_deref(), Some("lib"));

    session.cycle_panel(PanelCycle::Prev).expect("prev");
    assert_eq!(session.active_panel().as_deref(), Some("main"));
}

#[test]
fn from_tree_with_focus_named_panel() {
    let session: DockSession<u32> = DockSession::from_tree_with_focus(
        nested_layout(),
        InitialFocus::NamedPanel("props".into()),
    )
    .expect("session");
    let built = build_tree(&nested_layout()).expect("built");
    let props_panel = built.index.panel_node("props").expect("props");
    let props_pane = owning_pane(&built.layout, props_panel).expect("pane");

    assert_eq!(session.focused_pane(), Some(props_pane));
    assert_eq!(
        session.active_panel_in_pane(props_pane).as_deref(),
        Some("output")
    );
    assert_eq!(session.active_panel().as_deref(), Some("output"));
}

#[test]
fn clear_focus() {
    let session: DockSession<u32> = DockSession::from_tree(nested_layout()).expect("session");
    assert!(session.focused_pane().is_some());
    session.clear_focus();
    assert!(session.focused_pane().is_none());
}
