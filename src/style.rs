//! Styling for dock chrome: tabs, panes, splitters, and drop overlays.

use iced::widget::button;
use iced::{Background, Border, Color, Theme};

/// Complete style for the dock UI.
#[derive(Debug, Clone)]
pub struct DockStyle {
    /// Background behind the entire dock area (visible in gaps between panes).
    pub background: DockBackgroundStyle,
    /// Pane / window chrome (border, padding, fill).
    pub window: WindowStyle,
    /// Tab strip when a pane has multiple tabs.
    pub tab_bar: TabBarStyle,
    /// Individual tab appearance.
    pub tab: TabStyle,
    /// Splitter handles between panes.
    pub splitter: SplitterStyle,
    /// Highlight shown while dragging tabs over a drop target.
    pub drop_overlay: DropOverlayStyle,
}

/// Background fill for the dock root.
#[derive(Debug, Clone)]
pub struct DockBackgroundStyle {
    pub color: Color,
}

/// Pane window frame and content inset.
#[derive(Debug, Clone)]
pub struct WindowStyle {
    pub background: Color,
    pub border: Border,
    /// Border drawn when this pane has focus. Falls back to [`Self::border`] when `None`.
    pub focused_border: Option<Border>,
    pub padding: f32,
}

/// Close control on each tab.
#[derive(Debug, Clone)]
pub struct CloseButtonStyle {
    pub text_size: f32,
    /// Square width and height of the close control.
    pub size: f32,
    /// Space between the close control and the right edge of the tab.
    pub margin_right: f32,
    pub padding: [f32; 2],
    pub text_color: Color,
    pub background: Color,
    pub hovered_background: Color,
    pub hovered_text: Color,
    pub border_radius: f32,
}

/// Tab strip container.
#[derive(Debug, Clone)]
pub struct TabBarStyle {
    pub height: f32,
    pub background: Color,
    pub spacing: f32,
    /// Outer padding of the tab row: `[vertical, horizontal]`.
    pub padding: [f32; 2],
    /// Minimum pointer movement before a tab label press becomes a dock drag.
    pub drag_threshold: f32,
    pub close_button: CloseButtonStyle,
    /// Height of the floating horizontal scrollbar thumb when tabs overflow.
    pub scrollbar_height: f32,
    /// Scrollbar thumb color when the tab bar is hovered.
    pub scrollbar_thumb: Color,
    /// Scrollbar thumb color while the thumb is hovered or dragged.
    pub scrollbar_thumb_hovered: Color,
}

/// Individual tab label colors and padding.
#[derive(Debug, Clone)]
pub struct TabStyle {
    pub text_size: f32,
    /// Label padding: `[vertical, horizontal]`.
    pub padding: [f32; 2],
    pub border_radius: f32,
    pub inactive_background: Color,
    pub inactive_text: Color,
    pub hovered_background: Color,
    pub hovered_text: Color,
    /// Matches [`WindowStyle::background`] for the active tab.
    pub active_background: Color,
    pub active_text: Color,
    /// Bottom accent on the active tab.
    pub active_accent: Color,
}

/// Splitter handle between proportional children.
#[derive(Debug, Clone)]
pub struct SplitterStyle {
    pub size: f32,
    /// Extra space between panes (shows [`DockBackgroundStyle::color`]).
    pub gap: f32,
    /// Minimum width of each pane in horizontal split groups.
    ///
    /// Split drags stop when an adjacent pair would shrink a pane below this width.
    pub min_pane_width: f32,
    /// Minimum height of each pane in vertical split groups.
    ///
    /// Split drags stop when an adjacent pair would shrink a pane below this height.
    pub min_pane_height: f32,
    /// Drawn when idle (typically fully transparent).
    pub idle_color: Color,
    pub hover_color: Color,
    pub drag_color: Color,
}

/// Drop-zone highlight during tab drag.
#[derive(Debug, Clone)]
pub struct DropOverlayStyle {
    pub color: Color,
    /// Fraction of pane edge used for edge drop bands (0.0–0.5).
    pub edge_fraction: f32,
}

impl Default for DockStyle {
    fn default() -> Self {
        Self::from_theme(&Theme::Dark)
    }
}

impl DockStyle {
    /// Default dock chrome; uses the built-in [`Self::modern_dark`] palette.
    ///
    /// The `theme` argument is accepted for API compatibility with iced's `.style(|theme| …)`
    /// pattern but does not remap dock colors yet.
    pub fn from_theme(theme: &Theme) -> Self {
        let _ = theme;
        Self::modern_dark()
    }

    /// VS Code–inspired dark palette with flat panes and subtle chrome.
    pub fn modern_dark() -> Self {
        let canvas = Color::from_rgb(0.094, 0.094, 0.106); // #18181b — dock gaps / outer chrome
        let tab_bar_bg = Color::from_rgb(0.118, 0.118, 0.133); // #1e1e22 — tab strip
        let tab_inactive = Color::from_rgb(0.149, 0.149, 0.165); // #26262a — inactive tabs
        let pane = Color::from_rgb(0.145, 0.145, 0.157); // #252528 — pane / active tab
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
                drag_threshold: 6.0,
                close_button: CloseButtonStyle {
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
                scrollbar_height: 4.0,
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
                active_background: pane,
                active_text: text,
                active_accent: accent,
            },
            splitter: SplitterStyle {
                size: 0.5,
                gap: 10.0,
                min_pane_width: 80.0,
                min_pane_height: 80.0,
                idle_color: Color::TRANSPARENT,
                hover_color: Color::from_rgba(0.99, 0.99, 0.99, 0.99),
                drag_color: Color::from_rgba(0.99, 0.99, 0.99, 0.99),
            },
            drop_overlay: DropOverlayStyle {
                color: Color::from_rgba(0.38, 0.62, 0.98, 0.28),
                edge_fraction: 0.2,
            },
        }
    }

    /// Keep the active tab background aligned with the pane content background.
    pub fn sync_active_tab_with_window(&mut self) {
        self.tab.active_background = self.window.background;
    }

    /// Set the tab strip background to match [`DockBackgroundStyle::color`].
    pub fn sync_tab_bar_with_dock(&mut self) {
        self.tab_bar.background = self.background.color;
    }

    /// Keep the active tab fill aligned with the pane background.
    pub fn sync_tab_appearance(&mut self) {
        self.sync_active_tab_with_window();
    }

    /// Set the minimum pane width for horizontal splits.
    pub fn with_min_pane_width(mut self, min_pane_width: f32) -> Self {
        self.splitter.min_pane_width = min_pane_width.max(1.0);
        self
    }

    /// Set the minimum pane height for vertical splits.
    pub fn with_min_pane_height(mut self, min_pane_height: f32) -> Self {
        self.splitter.min_pane_height = min_pane_height.max(1.0);
        self
    }
}

/// Wrap a fixed [`DockStyle`] for use with [`crate::dock`]'s `.style(...)` builder.
pub fn constant(style: DockStyle) -> impl Fn(&Theme) -> DockStyle {
    move |_| style.clone()
}

/// Ghost close button for tab close controls.
pub fn close_button_style(
    close: &CloseButtonStyle,
) -> impl Fn(&Theme, button::Status) -> button::Style + Clone {
    let close = close.clone();
    move |_, status| {
        let (background, text_color) = match status {
            button::Status::Hovered | button::Status::Pressed => {
                (close.hovered_background, close.hovered_text)
            }
            _ => (close.background, close.text_color),
        };
        button::Style {
            background: if background.a > 0.0 {
                Some(Background::Color(background))
            } else {
                None
            },
            text_color,
            border: Border {
                radius: close.border_radius.into(),
                ..Border::default()
            },
            shadow: Default::default(),
            snap: false,
        }
    }
}
