//! Verifies tab selection marks layout dirty (the state transition that must not
//! trigger `rebuild_root` inside `Dock::update`).

use iced_dock::unstable::{build_tree, dispatch_action, first_pane, owning_pane};
use iced_dock::{panel, tabs, DockAction, DockWidgetState, TabAction};

fn welcome_file_state() -> DockWidgetState<u32> {
    let built = build_tree(
        &tabs([
            panel("welcome", "Welcome", 0u32),
            panel("file", "Document", 1u32),
        ])
        .active("welcome"),
    )
    .expect("valid layout");

    let focused_pane = first_pane(&built.layout);
    DockWidgetState::from_built(built, focused_pane)
}

#[test]
fn tab_select_sets_layout_dirty() {
    let mut state = welcome_file_state();
    state.layout_dirty = false;

    let file_panel = state.index.panel_node("file").expect("file panel");
    let pane = owning_pane(&state.layout, file_panel).expect("documents pane");

    let changed = dispatch_action(
        &mut state,
        DockAction::Tab(TabAction::Select {
            pane,
            panel: file_panel,
        }),
    );

    assert!(changed);
    assert!(
        state.layout_dirty,
        "tab select must request a layout pass instead of rebuilding in update"
    );
}
