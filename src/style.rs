//! Styling for dock chrome: title bars, tabs, panes, splitters, and drop overlays.

use iced::widget::button;
use iced::{Background, Border, Color, Theme};

/// Complete style for the dock UI.
#[derive(Debug, Clone)]
pub struct DockStyle {
    /// Background behind the entire dock area (visible in gaps between panes).
    pub background: DockBackgroundStyle,
    /// Pane / window chrome (border, padding, fill).
    pub window: WindowStyle,
    /// Title bar at the top of each pane.
    pub title_bar: TitleBarStyle,
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
    pub padding: f32,
    pub border_radius: f32,
}

/// Title bar metrics and colors.
#[derive(Debug, Clone)]
pub struct TitleBarStyle {
    pub height: f32,
    pub background: Color,
    pub text_color: Color,
    pub text_size: f32,
    pub close_button_width: f32,
    pub close_button: CloseButtonStyle,
    pub drag_threshold: f32,
}

/// Close control on the title bar (ghost style, not primary).
#[derive(Debug, Clone)]
pub struct CloseButtonStyle {
    pub text_size: f32,
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
    pub padding: [f32; 2],
}

/// Individual tab button colors and padding.
#[derive(Debug, Clone)]
pub struct TabStyle {
    pub text_size: f32,
    pub padding: [f32; 2],
    pub border_radius: f32,
    pub inactive_background: Color,
    pub inactive_text: Color,
    pub hovered_background: Color,
    pub hovered_text: Color,
    /// Matches [`WindowStyle::background`] for the active tab.
    pub active_background: Color,
    pub active_text: Color,
    pub active_accent: Color,
}

/// Splitter handle between proportional children.
#[derive(Debug, Clone)]
pub struct SplitterStyle {
    pub size: f32,
    /// Extra space between panes (shows [`DockBackgroundStyle::color`]).
    pub gap: f32,
    pub min_pane_size: f32,
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
    /// Build a cohesive, IDE-inspired dark style from the iced theme.
    pub fn from_theme(theme: &Theme) -> Self {
        let _ = theme;
        Self::modern_dark()
    }

    /// VS Code–inspired dark palette with rounded panes and subtle chrome.
    pub fn modern_dark() -> Self {
        let canvas = Color::from_rgb(0.094, 0.094, 0.106); // #18181b
        let pane = Color::from_rgb(0.145, 0.145, 0.157); // #252528
        let chrome_top = Color::from_rgb(0.133, 0.133, 0.145); // #222225
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
                padding: 0.0,
                border_radius: radius,
            },
            title_bar: TitleBarStyle {
                height: 32.0,
                background: chrome_top,
                text_color: text,
                text_size: 12.5,
                close_button_width: 40.0,
                close_button: CloseButtonStyle {
                    text_size: 15.0,
                    padding: [4.0, 10.0],
                    text_color: text_muted,
                    background: Color::TRANSPARENT,
                    hovered_background: Color::from_rgb(0.85, 0.25, 0.28),
                    hovered_text: Color::WHITE,
                    border_radius: 4.0,
                },
                drag_threshold: 6.0,
            },
            tab_bar: TabBarStyle {
                height: 30.0,
                background: canvas,
                spacing: 0.0,
                padding: [0.0, 0.0],
            },
            tab: TabStyle {
                text_size: 12.0,
                padding: [0.0, 10.0],
                border_radius: 0.0,
                inactive_background: Color::TRANSPARENT,
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
                min_pane_size: 80.0,
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

    /// Align tab strip fill with the dock root background.
    pub fn sync_tab_bar_with_dock(&mut self) {
        self.tab_bar.background = self.background.color;
    }

    /// Apply [`sync_tab_bar_with_dock`] and [`sync_active_tab_with_window`].
    pub fn sync_tab_appearance(&mut self) {
        self.sync_tab_bar_with_dock();
        self.sync_active_tab_with_window();
    }
}

/// Wrap a fixed [`DockStyle`] for use with [`crate::dock`]'s `.style(...)` builder.
pub fn constant(style: DockStyle) -> impl Fn(&Theme) -> DockStyle {
    move |_| style.clone()
}

/// Build an iced [`button::Style`] for a dock tab.
pub fn tab_button_style(
    tab: &TabStyle,
    active: bool,
) -> impl Fn(&Theme, button::Status) -> button::Style + Clone {
    let tab = tab.clone();
    move |_, status| {
        let (background, text_color, border) = if active {
            (
                Color::TRANSPARENT,
                tab.active_text,
                Border {
                    width: 0.0,
                    color: Color::TRANSPARENT,
                    radius: tab.border_radius.into(),
                },
            )
        } else {
            let (bg, text) = match status {
                button::Status::Hovered => (tab.hovered_background, tab.hovered_text),
                _ => (tab.inactive_background, tab.inactive_text),
            };
            (
                bg,
                text,
                Border {
                    width: 0.0,
                    color: Color::TRANSPARENT,
                    radius: tab.border_radius.into(),
                },
            )
        };
        button::Style {
            background: if background.a > 0.0 {
                Some(Background::Color(background))
            } else {
                None
            },
            text_color,
            border,
            shadow: Default::default(),
            snap: false,
        }
    }
}

/// Ghost close button for the title bar.
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
