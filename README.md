# iced_dock

A docking layout widget for [iced](https://github.com/iced-rs/iced) 0.14. Resizable splits, tabbed panes, drag-and-drop
docking, focus tracking, keyboard navigation.

![Digraph viewer: controls, byte rail, and matrix heatmap](assets/screenshot.png)

## Quick start

```toml
[dependencies]
iced_dock = { git = "https://github.com/Fee0/iced_dock.git" }
iced = { version = "0.14", features = ["wgpu"] }
```

```rust
use iced_dock::{dock, horizontal, panel, tabs, DockEvent, DockSession, LayoutTree};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Panel { Editor, Sidebar }

let tree: LayoutTree<Panel> = horizontal([
    tabs([panel("editor", "Editor", Panel::Editor)]),
    tabs([panel("sidebar", "Sidebar", Panel::Sidebar)]),
])
.weights([0.7, 0.3]);

let session = DockSession::from_tree(tree);

dock()
    .state(session.state())
    .on_event(Message::Dock)
    .content(|key| match key {
        Panel::Editor  => iced::widget::text("Editor").into(),
        Panel::Sidebar => iced::widget::text("Sidebar").into(),
    })
    .build()
```

## Layout builder

- `tabs([...])` — tabbed pane
- `horizontal([...])` / `vertical([...])` — split groups
- `single(panel(...))` — one panel filling the dock
- `.active("id")` — initial active tab
- `.named("name")` — register pane for `PaneTarget::Named`
- `.weights([...])` — split proportions

## Theming

Omit `.style(...)` to inherit the iced theme palette. Use presets for opinionated looks:

```rust
dock().style(iced_dock::preset::modern_dark()).build()
```

Per-pane overrides via `PaneContent::new(element).style(|_| DockStyle::modern_dark())`.

## DockSession

| Method                    | Purpose                             |
|---------------------------|-------------------------------------|
| `from_tree`               | Build from `LayoutTree`             |
| `dispatch(action)`        | Apply `DockAction` programmatically |
| `open_panel(target, def)` | Add and activate a panel            |
| `select_panel(id)`        | Activate tab + focus pane           |
| `close_panel(id)`         | Close tab, collapse empty panes     |
| `focus_adjacent(dir)`     | Move focus between panes            |
| `cycle_panel(cycle)`      | Next/prev tab in focused pane       |

## Events

`DockEvent` variants: `TabSelected`, `TabClosed`, `PaneFocused`, `SplitResized`, `DragStarted`, `DragMoved`,
`DragEnded`, `DragCancelled`, `LayoutChanged`.

## Keyboard navigation

The crate doesn't subscribe to keys. Use `session.focus_adjacent(Direction::Right)` after the first frame. See
`examples/minimal.rs` for `Ctrl+Arrow` setup.
