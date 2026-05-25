//! Built-in theme constructors and presets.

use iced::{Border, Color, Theme};

use super::{
    CloseButtonStyle, DockBackgroundStyle, DockStyle, DropOverlayStyle, SplitterStyle,
    TabBarSeparatorStyle, TabBarStyle, TabStyle, WindowStyle,
};

impl DockStyle {
    /// Dock chrome derived from the iced theme palette.
    ///
    /// Layout metrics match [`Self::modern_dark`]; colors come from
    /// [`Theme::extended_palette`]. This is the default for [`super::Catalog`] and
    /// [`crate::dock`] when no `.style(...)` is set.
    #[must_use]
    pub fn from_palette(theme: &Theme) -> Self {
        default(theme)
    }

    /// Built-in dock chrome for the given iced theme.
    #[deprecated(
        note = "use style::default(theme) for palette styling, or preset::modern_* for IDE chrome"
    )]
    #[must_use]
    pub fn from_theme(theme: &Theme) -> Self {
        default(theme)
    }

    /// VS Code–inspired dark preset (flat panes, subtle chrome).
    ///
    /// Not applied automatically — use [`preset::modern_dark`] with
    /// [`crate::dock::Dock::style`] to opt in.
    #[must_use]
    pub fn modern_dark() -> Self {
        let canvas = Color::from_rgb(0.094, 0.094, 0.106);
        let tab_bar_bg = Color::from_rgb(0.118, 0.118, 0.133);
        let tab_inactive = Color::from_rgb(0.149, 0.149, 0.165);
        let pane = Color::from_rgb(0.145, 0.145, 0.157);
        let border = Color::from_rgb(0.2, 0.2, 0.22);
        let text = Color::from_rgb(0.82, 0.82, 0.85);
        let text_muted = Color::from_rgb(0.55, 0.55, 0.58);
        let accent = Color::from_rgb(0.38, 0.62, 0.98);
        let radius = 0.0;

        Self {
            background: DockBackgroundStyle { color: canvas },
            window: WindowStyle {
                background: pane,
                border: Border {
                    width: 1.0,
                    color: border,
                    radius: radius.into(),
                },
                focused_border: Some(Border {
                    width: 1.0,
                    color: accent,
                    radius: radius.into(),
                }),
                padding: 0.0,
            },
            tab_bar: TabBarStyle {
                height: 30.0,
                background: tab_bar_bg,
                spacing: 0.0,
                padding: [0.0, 0.0],
                close_button: CloseButtonStyle {
                    label: "×".into(),
                    text_size: 15.0,
                    size: 20.0,
                    margin_right: 6.0,
                    padding: [0.0, 0.0],
                    text_color: text_muted,
                    background: Color::TRANSPARENT,
                    hovered_background: Color::from_rgb(0.85, 0.25, 0.28),
                    hovered_text: Color::WHITE,
                    border_radius: 3.0,
                },
                separator: Some(TabBarSeparatorStyle {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.35),
                    height: 1.0,
                }),
                scrollbar_height: 4.0,
                scrollbar_thumb_min_width: 2.0,
                scrollbar_thumb: Color::from_rgba(1.0, 1.0, 1.0, 0.28),
                scrollbar_thumb_hovered: Color::from_rgba(1.0, 1.0, 1.0, 0.45),
            },
            tab: TabStyle {
                text_size: 12.0,
                padding: [0.0, 10.0],
                border_radius: 0.0,
                inactive_background: tab_inactive,
                inactive_text: text_muted,
                hovered_background: Color::from_rgba(1.0, 1.0, 1.0, 0.06),
                hovered_text: text,
                pressed_background: Color::from_rgba(1.0, 1.0, 1.0, 0.1),
                pressed_text: text,
                active_background: pane,
                active_text: text,
                active_accent: accent,
                active_accent_height: 2.0,
            },
            splitter: SplitterStyle {
                size: 0.5,
                gap: 10.0,
                idle_color: Color::TRANSPARENT,
                hover_color: Color::from_rgba(0.99, 0.99, 0.99, 0.99),
                drag_color: Color::from_rgba(0.99, 0.99, 0.99, 0.99),
            },
            drop_overlay: DropOverlayStyle {
                color: Color::from_rgba(0.38, 0.62, 0.98, 0.28),
                insert_marker_width: 3.0,
                insert_marker_min_alpha: 0.65,
            },
        }
    }

    /// VS Code–inspired light preset.
    ///
    /// Not applied automatically — use [`preset::modern_light`] with
    /// [`crate::dock::Dock::style`] to opt in.
    #[must_use]
    pub fn modern_light() -> Self {
        let canvas = Color::from_rgb(0.92, 0.92, 0.94);
        let tab_bar_bg = Color::from_rgb(0.88, 0.88, 0.9);
        let tab_inactive = Color::from_rgb(0.84, 0.84, 0.87);
        let pane = Color::from_rgb(0.98, 0.98, 0.99);
        let border = Color::from_rgb(0.78, 0.78, 0.82);
        let text = Color::from_rgb(0.15, 0.15, 0.18);
        let text_muted = Color::from_rgb(0.45, 0.45, 0.5);
        let accent = Color::from_rgb(0.12, 0.45, 0.92);
        let radius = 0.0;

        Self {
            background: DockBackgroundStyle { color: canvas },
            window: WindowStyle {
                background: pane,
                border: Border {
                    width: 1.0,
                    color: border,
                    radius: radius.into(),
                },
                focused_border: Some(Border {
                    width: 1.0,
                    color: accent,
                    radius: radius.into(),
                }),
                padding: 0.0,
            },
            tab_bar: TabBarStyle {
                height: 30.0,
                background: tab_bar_bg,
                spacing: 0.0,
                padding: [0.0, 0.0],
                close_button: CloseButtonStyle {
                    label: "×".into(),
                    text_size: 15.0,
                    size: 20.0,
                    margin_right: 6.0,
                    padding: [0.0, 0.0],
                    text_color: text_muted,
                    background: Color::TRANSPARENT,
                    hovered_background: Color::from_rgb(0.85, 0.25, 0.28),
                    hovered_text: Color::WHITE,
                    border_radius: 3.0,
                },
                separator: Some(TabBarSeparatorStyle {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.12),
                    height: 1.0,
                }),
                scrollbar_height: 4.0,
                scrollbar_thumb_min_width: 2.0,
                scrollbar_thumb: Color::from_rgba(0.0, 0.0, 0.0, 0.22),
                scrollbar_thumb_hovered: Color::from_rgba(0.0, 0.0, 0.0, 0.38),
            },
            tab: TabStyle {
                text_size: 12.0,
                padding: [0.0, 10.0],
                border_radius: 0.0,
                inactive_background: tab_inactive,
                inactive_text: text_muted,
                hovered_background: Color::from_rgba(0.0, 0.0, 0.0, 0.05),
                hovered_text: text,
                pressed_background: Color::from_rgba(0.0, 0.0, 0.0, 0.08),
                pressed_text: text,
                active_background: pane,
                active_text: text,
                active_accent: accent,
                active_accent_height: 2.0,
            },
            splitter: SplitterStyle {
                size: 0.5,
                gap: 10.0,
                idle_color: Color::TRANSPARENT,
                hover_color: Color::from_rgba(0.2, 0.2, 0.25, 0.35),
                drag_color: Color::from_rgba(0.2, 0.2, 0.25, 0.5),
            },
            drop_overlay: DropOverlayStyle {
                color: Color::from_rgba(0.12, 0.45, 0.92, 0.25),
                insert_marker_width: 3.0,
                insert_marker_min_alpha: 0.65,
            },
        }
    }
}

/// The default [`DockStyle`] for a [`Theme`], using [`Theme::extended_palette`].
#[must_use]
pub fn default(theme: &Theme) -> DockStyle {
    let palette = theme.extended_palette();
    let canvas = palette.background.base.color;
    let tab_bar_bg = palette.background.weaker.color;
    let tab_inactive = palette.background.neutral.color;
    let pane = palette.background.base.color;
    let border = palette.background.strong.color;
    let text = palette.background.base.text;
    let text_muted = palette.background.weak.text;
    let accent = palette.primary.base.color;
    let accent_strong = palette.primary.strong.color;
    let hover_overlay = if palette.is_dark {
        Color::from_rgba(1.0, 1.0, 1.0, 0.06)
    } else {
        Color::from_rgba(0.0, 0.0, 0.0, 0.05)
    };
    let pressed_overlay = if palette.is_dark {
        Color::from_rgba(1.0, 1.0, 1.0, 0.1)
    } else {
        Color::from_rgba(0.0, 0.0, 0.0, 0.08)
    };
    let separator_alpha = if palette.is_dark { 0.35 } else { 0.12 };
    let scrollbar_thumb = palette.background.strong.color.scale_alpha(0.35);
    let scrollbar_thumb_hovered = palette.background.strong.color.scale_alpha(0.55);
    let splitter_hover = palette.primary.base.color;
    let splitter_drag = palette.primary.strong.color;
    let radius = 0.0;

    let mut style = DockStyle::modern_dark();
    style.background.color = canvas;
    style.window = WindowStyle {
        background: pane,
        border: Border {
            width: 1.0,
            color: border,
            radius: radius.into(),
        },
        focused_border: Some(Border {
            width: 1.0,
            color: accent_strong,
            radius: radius.into(),
        }),
        padding: 0.0,
    };
    style.tab_bar.background = tab_bar_bg;
    style.tab_bar.close_button.text_color = text_muted;
    style.tab_bar.separator = Some(TabBarSeparatorStyle {
        color: Color::from_rgba(0.0, 0.0, 0.0, separator_alpha),
        height: 1.0,
    });
    style.tab_bar.scrollbar_thumb = scrollbar_thumb;
    style.tab_bar.scrollbar_thumb_hovered = scrollbar_thumb_hovered;
    style.tab.inactive_background = tab_inactive;
    style.tab.inactive_text = text_muted;
    style.tab.hovered_background = hover_overlay;
    style.tab.hovered_text = text;
    style.tab.pressed_background = pressed_overlay;
    style.tab.pressed_text = text;
    style.tab.active_background = pane;
    style.tab.active_text = text;
    style.tab.active_accent = accent;
    style.splitter.hover_color = splitter_hover;
    style.splitter.drag_color = splitter_drag;
    style.drop_overlay.color = accent.scale_alpha(0.28);
    style
}

/// Optional IDE-style presets (ignore the active iced theme).
pub mod preset {
    use crate::style::{DockStyle, StyleFn};
    use iced::Theme;

    /// VS Code–inspired dark chrome regardless of [`Theme`].
    #[must_use]
    pub fn modern_dark() -> StyleFn<'static, Theme> {
        Box::new(|_| DockStyle::modern_dark())
    }

    /// VS Code–inspired light chrome regardless of [`Theme`].
    #[must_use]
    pub fn modern_light() -> StyleFn<'static, Theme> {
        Box::new(|_| DockStyle::modern_light())
    }
}
