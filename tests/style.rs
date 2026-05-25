use iced::Theme;
use iced_dock::{constant, default, preset, Catalog, DockStyle};

#[test]
fn palette_default_has_sane_metrics() {
    let style = default(&Theme::Dark);
    assert!(style.tab_bar.height > 0.0);
    assert!(style.tab_bar.scrollbar_height > 0.0);
    assert!(style.tab_bar.close_button.size > 0.0);
    assert!(style.splitter.size > 0.0);
    assert!(style.splitter.gap > 0.0);
    assert!(style.window.border.width >= 0.0);
}

#[test]
fn catalog_default_matches_style_default() {
    let theme = Theme::Dark;
    let from_catalog = Catalog::style(&theme, &<Theme as Catalog>::default());
    let from_fn = default(&theme);
    assert_eq!(from_catalog.tab.active_accent, from_fn.tab.active_accent);
    assert_eq!(from_catalog.window.background, from_fn.window.background);
}

#[test]
fn palette_default_uses_primary_for_accent_and_split_hover() {
    let theme = Theme::Dark;
    let style = default(&theme);
    let palette = theme.extended_palette();
    assert_eq!(style.tab.active_accent, palette.primary.base.color);
    assert_eq!(style.splitter.hover_color, palette.primary.base.color);
    assert_eq!(style.splitter.drag_color, palette.primary.strong.color);
}

#[test]
fn tab_bar_and_inactive_tabs_differ_from_dock_background() {
    let style = default(&Theme::Dark);
    assert_ne!(style.tab_bar.background, style.background.color);
    assert_ne!(style.tab.inactive_background, style.background.color);
}

#[test]
fn active_tab_matches_window_background() {
    let mut style = default(&Theme::Dark);
    style.window.background = iced::Color::from_rgb(0.1, 0.2, 0.3);
    style.sync_active_tab_with_window();
    assert_eq!(style.tab.active_background, style.window.background);
}

#[test]
fn constant_style_helper() {
    let custom = DockStyle::modern_dark();
    let resolved = constant(custom.clone())(&Theme::Light);
    assert_eq!(resolved.tab_bar.height, custom.tab_bar.height);
}

#[test]
fn idle_splitter_is_transparent_by_default() {
    let style = default(&Theme::Dark);
    assert_eq!(style.splitter.idle_color.a, 0.0);
}

#[test]
fn palette_default_has_focused_border() {
    let style = default(&Theme::Dark);
    assert!(style.window.focused_border.is_some());
}

#[test]
fn splitter_style_has_visual_fields_only() {
    let style = default(&Theme::Dark);
    assert!(style.splitter.size > 0.0);
    assert!(style.splitter.gap > 0.0);
}

#[test]
fn preset_modern_light_differs_from_palette_default() {
    let light_preset = preset::modern_light()(&Theme::Light);
    let palette = default(&Theme::Light);
    assert_ne!(light_preset.background.color, palette.background.color);
    assert_ne!(light_preset.tab_bar.background, palette.tab_bar.background);
}

#[test]
fn preset_modern_dark_matches_modern_dark_constructor() {
    let from_preset = preset::modern_dark()(&Theme::Light);
    let direct = DockStyle::modern_dark();
    assert_eq!(from_preset.background.color, direct.background.color);
    assert_eq!(from_preset.tab_bar.background, direct.tab_bar.background);
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
