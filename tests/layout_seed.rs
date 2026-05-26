use iced_dock::widget::DockWidgetState;

#[test]
fn default_layout_is_empty() {
    let state = DockWidgetState::<u32>::default();
    assert!(
        state.layout.root_child().is_none(),
        "default dock state should start with an empty layout"
    );
}
