//! Regression: a single close/select must not apply layout mutations twice.

use iced_dock::unstable::{build_tree, dispatch_action};
use iced_dock::{
    panel, tabs, ContentKey, DockAction, DockSession, DockWidgetState, TabAction,
};

#[test]
fn dispatch_close_once_removes_panel() {
    let built = build_tree(&tabs([
        panel("a", "A", ContentKey(0)),
        panel("b", "B", ContentKey(1)),
    ]))
    .expect("built");
    let panel_b = built.index.panel_node("b").expect("b");
    let mut state = DockWidgetState::<iced::Theme>::from_built(built, None);

    assert!(dispatch_action(
        &mut state,
        DockAction::Tab(TabAction::Close { panel: panel_b })
    ));
    assert!(!state.index.panels.contains_key("b"));
    assert!(state.index.panels.contains_key("a"));

    let second = dispatch_action(
        &mut state,
        DockAction::Tab(TabAction::Close { panel: panel_b })
    );
    assert!(!second);
}

#[test]
fn session_select_does_not_require_update_handler() {
    let session: DockSession = DockSession::from_tree(tabs([
        panel("a", "A", ContentKey(0)),
        panel("b", "B", ContentKey(1)),
    ]))
    .expect("session");
    session.select_panel("b").expect("select");
    assert_eq!(session.active_panel().as_deref(), Some("b"));
    let count_after_one = session.state().borrow().layout.nodes.len();
    session.select_panel("a").expect("select again");
    let count_after_two = session.state().borrow().layout.nodes.len();
    assert_eq!(count_after_one, count_after_two);
}
