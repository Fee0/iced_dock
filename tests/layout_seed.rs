use iced_dock::widget::DockWidgetState;

#[test]
fn default_layout_has_root_child() {
    let state = DockWidgetState::default();
    assert!(
        state.layout.root_child().is_some(),
        "IDE seed layout should set a root proportional node"
    );
}
