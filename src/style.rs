//! Styling for dock chrome: tabs, panes, splitters, and drop overlays.

pub mod theme;
pub use theme::{default, preset};

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
    /// Label shown on the close control (default `"×"`).
    pub label: String,
    pub text_size: f32,
    /// Square width and height of the close control.
    pub size: f32,
    /// Space between the close control and the right edge of the tab.
    pub margin_right: f32,
    /// Inner padding of the close control: `[vertical, horizontal]`.
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
    pub close_button: CloseButtonStyle,
    /// Line drawn along the bottom of the tab strip; `None` disables it.
    pub separator: Option<TabBarSeparatorStyle>,
    /// Height of the floating horizontal scrollbar thumb when tabs overflow.
    pub scrollbar_height: f32,
    /// Minimum width of the scrollbar thumb.
    pub scrollbar_thumb_min_width: f32,
    /// Scrollbar thumb color when the tab bar is hovered.
    pub scrollbar_thumb: Color,
    /// Scrollbar thumb color while the thumb is hovered or dragged.
    pub scrollbar_thumb_hovered: Color,
}

/// Bottom edge line on the tab strip.
#[derive(Debug, Clone)]
pub struct TabBarSeparatorStyle {
    pub color: Color,
    pub height: f32,
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
    pub pressed_background: Color,
    pub pressed_text: Color,
    /// Matches [`WindowStyle::background`] for the active tab.
    pub active_background: Color,
    pub active_text: Color,
    /// Bottom accent on the active tab.
    pub active_accent: Color,
    /// Height of the active tab bottom accent bar.
    pub active_accent_height: f32,
}

/// Splitter handle between proportional children.
#[derive(Debug, Clone)]
pub struct SplitterStyle {
    pub size: f32,
    /// Extra space between panes (shows [`DockBackgroundStyle::color`]).
    pub gap: f32,
    /// Drawn when idle (typically fully transparent).
    pub idle_color: Color,
    pub hover_color: Color,
    pub drag_color: Color,
}

/// Drop-zone highlight during tab drag.
#[derive(Debug, Clone)]
pub struct DropOverlayStyle {
    pub color: Color,
    /// Width of the vertical insertion marker in the tab bar during drag.
    pub insert_marker_width: f32,
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

    /// Keep the active tab fill aligned with the pane background.
    pub fn sync_tab_appearance(&mut self) {
        self.sync_active_tab_with_window();
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
    pub element: iced::Element<'a, Message, Theme, Renderer>,
    pub style: Option<<Theme as Catalog>::Class<'static>>,
}

impl<'a, Message, Theme, Renderer> PaneContent<'a, Message, Theme, Renderer>
where
    Theme: Catalog,
{
    pub fn new(element: impl Into<iced::Element<'a, Message, Theme, Renderer>>) -> Self {
        Self {
            element: element.into(),
            style: None,
        }
    }

    /// Override the dock-level style for this pane.
    pub fn style(mut self, style: impl Fn(&Theme) -> DockStyle + 'static) -> Self
    where
        <Theme as Catalog>::Class<'static>: From<StyleFn<'static, Theme>>,
    {
        self.style = Some((Box::new(style) as StyleFn<'static, Theme>).into());
        self
    }

    /// Override the dock-level style class for this pane.
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

/// Ghost close button for tab close controls.
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
            shadow: Default::default(),
            snap: false,
        }
    }
}
