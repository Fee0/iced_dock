use iced::Theme;
use iced_dock::{constant, DockStyle};

#[test]
fn default_style_from_theme_has_sane_metrics() {
    let style = DockStyle::from_theme(&Theme::Dark);
    assert!(style.title_bar.height > 0.0);
    assert!(style.tab_bar.height > 0.0);
    assert!(style.tab_bar.scrollbar_height > 0.0);
    assert!(style.splitter.size > 0.0);
    assert!(style.splitter.gap > 0.0);
    assert!(style.window.border_radius >= 0.0);
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
    assert_eq!(resolved.title_bar.height, custom.title_bar.height);
}

#[test]
fn idle_splitter_is_transparent_by_default() {
    let style = DockStyle::from_theme(&Theme::Dark);
    assert_eq!(style.splitter.idle_color.a, 0.0);
}

#[test]
fn with_min_pane_width_and_height_update_splitter_style() {
    let style = DockStyle::from_theme(&Theme::Dark)
        .with_min_pane_width(120.0)
        .with_min_pane_height(64.0);
    assert_eq!(style.splitter.min_pane_width, 120.0);
    assert_eq!(style.splitter.min_pane_height, 64.0);
}
