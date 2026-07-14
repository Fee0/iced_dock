//! Styling for dock chrome: tabs, panes, splitters, and drop overlays.

pub mod theme;
pub use theme::{default, preset};

use iced::widget::{button, svg};
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
    /// Fill color visible in gaps between panes and behind splitters.
    pub color: Color,
}

/// Pane window frame.
#[derive(Debug, Clone)]
pub struct WindowStyle {
    /// Content area background fill.
    pub background: Color,
    /// Border drawn around each pane.
    pub border: Border,
    /// Border drawn when this pane has focus. Falls back to [`Self::border`] when `None`.
    pub focused_border: Option<Border>,
}

/// Close control on each tab (paint only).
#[derive(Debug, Clone)]
pub struct CloseButtonStyle {
    /// Text color in the default (idle) state.
    pub text_color: Color,
    /// Background color in the default (idle) state.
    pub background: Color,
    /// Background color when the close button is hovered.
    pub hovered_background: Color,
    /// Text color when the close button is hovered.
    pub hovered_text: Color,
    /// Corner radius applied to the close button.
    pub border_radius: f32,
}

/// Tab strip container (paint only).
#[derive(Debug, Clone)]
pub struct TabBarStyle {
    /// Background fill behind the tab strip row.
    pub background: Color,
    /// Appearance of the per-tab close button.
    pub close_button: CloseButtonStyle,
    /// Separator color drawn along the bottom of the tab strip; `None` disables it.
    pub separator: Option<Color>,
    /// Scrollbar track color.
    pub scrollbar_track: Color,
    /// Scrollbar thumb color when the tab bar is hovered.
    pub scrollbar_thumb: Color,
    /// Scrollbar thumb color while the thumb is hovered or dragged.
    pub scrollbar_thumb_hovered: Color,
    /// Scrollbar thumb border color.
    pub scrollbar_thumb_border: Color,
}

/// Individual tab label colors.
#[derive(Debug, Clone)]
pub struct TabStyle {
    /// Corner radius applied to tab labels.
    pub border_radius: f32,
    /// Background color of non-selected tabs.
    pub inactive_background: Color,
    /// Text color of non-selected tabs.
    pub inactive_text: Color,
    /// Background color when a tab is hovered.
    pub hovered_background: Color,
    /// Text color when a tab is hovered.
    pub hovered_text: Color,
    /// Background color while a tab is pressed (mouse down).
    pub pressed_background: Color,
    /// Text color while a tab is pressed.
    pub pressed_text: Color,
    /// Background color of the selected tab. Typically matches
    /// [`WindowStyle::background`] so the tab blends into the pane content.
    pub active_background: Color,
    /// Text color of the selected tab.
    pub active_text: Color,
    /// Bottom accent bar color on the selected tab.
    pub active_accent: Color,
}

/// Splitter handle colors.
#[derive(Debug, Clone)]
pub struct SplitterStyle {
    /// Drawn when idle (typically fully transparent).
    pub idle_color: Color,
    /// Color when the pointer hovers over the splitter.
    pub hover_color: Color,
    /// Color while the splitter is being dragged.
    pub drag_color: Color,
}

/// Drop-zone highlight during tab drag.
#[derive(Debug, Clone)]
pub struct DropOverlayStyle {
    /// Highlight color for valid drop zones.
    pub color: Color,
    /// Color shown when the drag would be rejected (e.g. group mismatch).
    pub blocked_color: Color,
    /// Minimum alpha for the tab-bar insertion marker (derived from [`Self::color`]).
    pub insert_marker_min_alpha: f32,
}

impl DockStyle {
    /// Keep the active tab background aligned with the pane content background.
    pub fn sync_active_tab_with_window(&mut self) {
        self.tab.active_background = self.window.background;
    }

    /// Set the tab strip background to match [`DockBackgroundStyle::color`].
    pub fn sync_tab_bar_with_dock(&mut self) {
        self.tab_bar.background = self.background.color;
    }

}

/// The theme catalog of dock chrome.
pub trait Catalog {
    /// The style class of this [`Catalog`].
    type Class<'a>;

    /// The default class produced by this [`Catalog`].
    fn default<'a>() -> Self::Class<'a>;

    /// The [`DockStyle`] for a class.
    fn style(&self, class: &Self::Class<'_>) -> DockStyle;
}

/// A styling function for dock chrome.
pub type StyleFn<'a, Theme> = Box<dyn Fn(&Theme) -> DockStyle + 'a>;

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(theme::default)
    }

    fn style(&self, class: &Self::Class<'_>) -> DockStyle {
        class(self)
    }
}

/// Wrap a fixed [`DockStyle`] for use with [`crate::dock`]'s `.style(...)` builder.
#[must_use]
pub fn constant<T>(style: DockStyle) -> StyleFn<'static, T> {
    Box::new(move |_| style.clone())
}

/// Pane content with an optional per-pane style override.
///
/// Returned by the content closure passed to [`crate::dock`]. Use [`From<Element>`]
/// for the common case (no override), or [`PaneContent::new`] + `.style(...)` for
/// per-pane chrome.
pub struct PaneContent<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Theme: Catalog,
{
    /// The widget tree rendered inside this pane.
    pub element: iced::Element<'a, Message, Theme, Renderer>,
    /// Optional per-pane style override. When `None`, the dock-level style applies.
    pub style: Option<<Theme as Catalog>::Class<'static>>,
}

impl<'a, Message, Theme, Renderer> PaneContent<'a, Message, Theme, Renderer>
where
    Theme: Catalog,
{
    /// Wrap an element as pane content with no style override.
    pub fn new(element: impl Into<iced::Element<'a, Message, Theme, Renderer>>) -> Self {
        Self {
            element: element.into(),
            style: None,
        }
    }

    /// Override the dock-level style for this pane.
    #[must_use]
    pub fn style(mut self, style: impl Fn(&Theme) -> DockStyle + 'static) -> Self
    where
        <Theme as Catalog>::Class<'static>: From<StyleFn<'static, Theme>>,
    {
        self.style = Some((Box::new(style) as StyleFn<'static, Theme>).into());
        self
    }

    /// Override the dock-level style class for this pane.
    #[must_use]
    pub fn class(mut self, class: <Theme as Catalog>::Class<'static>) -> Self {
        self.style = Some(class);
        self
    }
}

impl<'a, Message, Theme, Renderer> From<iced::Element<'a, Message, Theme, Renderer>>
    for PaneContent<'a, Message, Theme, Renderer>
where
    Theme: Catalog,
{
    fn from(element: iced::Element<'a, Message, Theme, Renderer>) -> Self {
        Self {
            element,
            style: None,
        }
    }
}

/// Convert a [`CloseButtonStyle`] into an iced SVG [`svg::Style`] closure
/// suitable for use with [`iced::widget::Svg::style`].
pub fn close_icon_style<T>(
    close: &CloseButtonStyle,
) -> impl Fn(&T, svg::Status) -> svg::Style + Clone {
    let close = close.clone();
    move |_, status| svg::Style {
        color: Some(match status {
            svg::Status::Hovered => close.hovered_text,
            svg::Status::Idle => close.text_color,
        }),
    }
}

/// Convert a [`CloseButtonStyle`] into an iced [`button::Style`] closure
/// suitable for use with [`iced::widget::Button::style`].
pub fn close_button_style<T>(
    close: &CloseButtonStyle,
) -> impl Fn(&T, button::Status) -> button::Style + Clone {
    let close = close.clone();
    move |_, status| {
        let (background, text_color) = match status {
            button::Status::Hovered | button::Status::Pressed => {
                (close.hovered_background, close.hovered_text)
            }
            _ => (close.background, close.text_color),
        };
        button::Style {
            background: (background.a > 0.0).then_some(Background::Color(background)),
            text_color,
            border: Border {
                radius: close.border_radius.into(),
                ..Border::default()
            },
            shadow: iced::Shadow::default(),
            snap: false,
        }
    }
}
