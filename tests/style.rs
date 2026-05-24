use iced::Theme;
use iced_dock::{constant, DockStyle};

#[test]
fn default_style_from_theme_has_sane_metrics() {
    let style = DockStyle::from_theme(&Theme::Dark);
    assert!(style.tab_bar.height > 0.0);
    assert!(style.tab_bar.drag_threshold > 0.0);
    assert!(style.tab_bar.scrollbar_height > 0.0);
    assert!(style.tab_bar.close_button.size > 0.0);
    assert!(style.splitter.size > 0.0);
    assert!(style.splitter.gap > 0.0);
    assert!(style.window.border.width >= 0.0);
}

#[test]
fn tab_bar_and_inactive_tabs_differ_from_dock_background() {
    let style = DockStyle::from_theme(&Theme::Dark);
    assert_ne!(style.tab_bar.background, style.background.color);
    assert_ne!(style.tab.inactive_background, style.background.color);
}

#[test]
fn active_tab_matches_window_background() {
    let mut style = DockStyle::from_theme(&Theme::Dark);
    style.window.background = iced::Color::from_rgb(0.1, 0.2, 0.3);
    style.sync_active_tab_with_window();
    assert_eq!(style.tab.active_background, style.window.background);
}

#[test]
fn constant_style_helper() {
    let custom = DockStyle::default();
    let resolved = constant(custom.clone())(&Theme::Light);
    assert_eq!(resolved.tab_bar.height, custom.tab_bar.height);
}

#[test]
fn idle_splitter_is_transparent_by_default() {
    let style = DockStyle::from_theme(&Theme::Dark);
    assert_eq!(style.splitter.idle_color.a, 0.0);
}

#[test]
fn modern_dark_has_focused_border() {
    let style = DockStyle::from_theme(&Theme::Dark);
    assert!(style.window.focused_border.is_some());
}

#[test]
fn with_min_pane_width_and_height_update_splitter_style() {
    let style = DockStyle::from_theme(&Theme::Dark)
        .with_min_pane_width(120.0)
        .with_min_pane_height(64.0);
    assert_eq!(style.splitter.min_pane_width, 120.0);
    assert_eq!(style.splitter.min_pane_height, 64.0);
}

#[test]
fn from_theme_light_uses_modern_light_palette() {
    let light = DockStyle::from_theme(&Theme::Light);
    let dark = DockStyle::modern_dark();
    assert_ne!(light.background.color, dark.background.color);
    assert_ne!(light.tab_bar.background, dark.tab_bar.background);
}

#[test]
fn modern_dark_exposes_separator_and_insert_marker_fields() {
    let style = DockStyle::modern_dark();
    assert!(style.tab_bar.separator.is_some());
    assert!(style.tab.active_accent_height > 0.0);
    assert!(style.drop_overlay.insert_marker_width > 0.0);
    assert_eq!(style.tab_bar.close_button.label, "×");
}

#[test]
fn tab_separator_can_be_disabled() {
    let mut style = DockStyle::modern_dark();
    style.tab_bar.separator = None;
    assert!(style.tab_bar.separator.is_none());
}
