# iced_dock

A docking layout widget for [iced](https://github.com/iced-rs/iced) 0.14. Build IDE-style UIs with resizable split panes, tabbed document areas, drag-and-drop tab docking, and pane focus — all integrated with iced's widget tree.

## Features

### Layout

- **Declarative layout trees** — describe your dock as nested horizontal/vertical splits and tabbed panes using a small builder API (`horizontal`, `vertical`, `tabs`, `panel`).
- **Runtime layout graph** — splits, tab order, and active tabs live in a mutable `Layout` backed by stable `NodeId` handles (slotmap arena).
- **Proportional splits** — resize panes by dragging splitters; optional initial weights per split group.
- **Minimum pane sizes** — configurable minimum width/height so splits cannot collapse panes below usable size.

### Tabs

- **Tabbed panes** — each pane hosts one or more panels (tabs) with a single active tab visible at a time.
- **Tab bar** — click to select, close button per tab (when enabled), horizontal scroll for overflow (mouse wheel and optional scrollbar thumb).
- **Tab drag-and-drop** — drag tabs to reorder within a pane, move to another pane's tab bar, or dock into content drop zones.
- **Per-panel flags** — `can_close`, `can_drag`, and `can_drop` on each panel definition.

### Drag-and-drop docking

- **Content drop zones** — while dragging a tab, edge bands (left/right/top/bottom) and a center zone on target panes show where the panel will land.
- **Drop operations** — center fill replaces/joins tabs; edge drops split the target pane in that direction.
- **Tab bar insertion** — dropping on a tab strip inserts at the hovered position (takes priority over content zones).
- **Cross-pane moves** — panels can move between panes; empty panes collapse and the tree is simplified automatically.

### Pane focus

- **Focused pane** — one pane has global focus at a time (accent border), separate from which tab is active inside each pane.
- **Click to focus** — clicking a pane's content area focuses that pane and emits `DockMessage::PaneFocused`.
- **Tab click** — selects the tab and focuses its pane.
- **Keyboard navigation** — `adjacent_pane` helper finds the nearest neighbor in a direction (gap-tolerant); wire it to `Ctrl+Arrow` or similar in your app.
- **Pane targets** — open new panels into the focused pane (`PaneTarget::Active`), a named pane, or the first pane in tree order.

### Styling

- **VS Code–inspired dark theme** — `DockStyle::modern_dark()` / `DockStyle::from_theme`.
- **Customizable chrome** — pane borders (including focused accent), tab colors, splitter handles, drop overlays, tab bar metrics, close buttons.
- **Builder overrides** — `min_pane_width`, `min_pane_height`, tab bar scrollbar visibility and hide delay.

### Integration

- **iced widget** — `dock()` builder returns a `Dock` widget that plugs into any iced `Element` tree.
- **Event messages** — `DockMessage` / `TabMessage` for tab select, close, drag lifecycle, split resize, pane focus, and layout changes.
- **Automatic state updates** — layout mutations from dock messages are applied internally; your app can observe events and optionally call `DockSession::apply_message`.
- **Content mapping** — you provide a `ContentKey → Element` function; the dock resolves which panel is visible per pane.

### Runtime API

- **`DockSession`** — high-level handle for opening, focusing, and closing panels by string id; tracks panel/pane indexes.
- **`DockWidgetState`** — shared state for the widget (`layout`, drag session, focused pane); usable with or without `DockSession`.
- **`Factory`** — low-level layout mutations (splits, tab moves, close) for advanced use.

### Persistence (optional)

- **`serde` feature** — serialize/deserialize declarative `LayoutTree` and runtime `Layout` to save user layouts and restore split/tab state.

## Quick start

Add to `Cargo.toml`:

```toml
[dependencies]
iced_dock = { path = ".." }  # or from git/crates.io when published
iced = { version = "0.14", features = ["wgpu"] }
```

Define a layout and wire the widget:

```rust
use iced::widget::text;
use iced::{Element, Theme};
use iced_dock::{
    dock, horizontal, panel, tabs, ContentKey, DockMessage, DockSession, LayoutTree,
};

fn layout() -> LayoutTree {
    horizontal([
        tabs([
            panel("main", "main.rs", ContentKey(0)),
            panel("lib", "lib.rs", ContentKey(1)),
        ])
        .active("main"),
        tabs([panel("preview", "preview", ContentKey(2))]),
    ])
    .weights([0.7, 0.3])
}

struct App {
    dock: DockSession,
}

enum Message {
    Dock(DockMessage),
}

fn view(app: &App) -> Element<'_, Message> {
    dock::<Message>()
        .state(app.dock.state())
        .on_event(Message::Dock)
        .content(panel_content)
        .build()
        .into()
}

fn panel_content(key: ContentKey) -> Element<'static, Message> {
    text(format!("Panel {}", key.0)).into()
}

fn update(app: &mut App, message: Message) {
    if let Message::Dock(msg) = message {
        app.dock.apply_message(msg);
    }
}
```

Run the full demo (multi-split IDE layout, focus border, `Ctrl+Arrow` navigation):

```bash
cargo run --example minimal
```

## Core concepts

| Concept | Description |
|---------|-------------|
| **Panel** | A tab: string id, title, and `ContentKey` that maps to your UI. |
| **Pane** | A tab group showing one active panel; has optional name for `PaneTarget::Named`. |
| **Active tab** | Which panel is visible inside a pane (`Pane.active`). |
| **Focused pane** | Which pane last received attention globally (`DockWidgetState.focused_pane`). |
| **ContentKey** | Opaque `u32` you use to build the correct `Element` for a panel. |
| **LayoutTree** | Declarative spec compiled once into runtime `Layout`. |

Tab selection and pane focus are separate (same model as iced's `pane_grid`): a pane can show an active tab while focus moves between panes for commands like "open in active pane".

## Layout builder

```rust
use iced_dock::{horizontal, vertical, tabs, panel, single, ContentKey, LayoutTree};

let tree: LayoutTree = horizontal([
    vertical([
        tabs([panel("editor", "Editor", ContentKey(0))]).named("editor"),
        tabs([panel("term", "Terminal", ContentKey(1))]),
    ])
    .weights([0.75, 0.25]),
    tabs([panel("sidebar", "Sidebar", ContentKey(2))]),
])
.weights([0.8, 0.2]);
```

Helpers:

- `tabs([...])` — tabbed pane
- `horizontal([...])` / `vertical([...])` — split groups
- `single(panel(...))` — one panel filling the dock
- `.active("panel_id")` — initial active tab
- `.named("pane_name")` — register pane for `PaneTarget::Named`
- `.weights([...])` — initial split proportions

Compile standalone with `build_tree(&tree)` or use `DockSession::from_tree(tree)`.

## Dock widget builder

```rust
dock::<Message>()
    .state(session.state())           // shared Rc<RefCell<DockWidgetState>>
    .on_event(Message::Dock)          // required: map DockMessage to app Message
    .content(fn(ContentKey) -> Element) // required: panel content factory
    .style(|theme| DockStyle::from_theme(theme))
    .min_pane_width(200.0)
    .min_pane_height(120.0)
    .tab_bar_show_scrollbar(false)
    .tab_bar_scrollbar_hide_delay(Duration::from_secs(1))
    .drag_active(false)               // visual hint during tab drag
    .build()
```

## DockSession

| Method | Purpose |
|--------|---------|
| `from_tree` | Build session from `LayoutTree` |
| `state()` | Shared state for the widget |
| `apply_message` | Apply `DockMessage` and refresh indexes |
| `open_panel(target, def)` | Add and activate a panel |
| `focus_panel(id)` | Activate tab by panel id and focus its pane |
| `focus_pane(node_id)` | Focus pane without changing active tab |
| `close_panel(id)` | Close tab and collapse empty panes |
| `focused_pane()` | Current focused pane `NodeId` |
| `active_panel()` | Active tab id in the focused pane |
| `panel_ids()` | All registered panel ids |

`PaneTarget`: `Active` (focused pane), `Named("pane_name")`, `First`.

## Events

`DockMessage` variants:

- `Tab(Select { pane, panel })` — tab clicked
- `Tab(Close { panel })` — close button pressed
- `Tab(DragStarted / DragMoved / DragEnded / DragCancelled)` — tab drag lifecycle
- `PaneFocused { pane, panel }` — content click or programmatic focus
- `SplitDrag { group, splitter_index, pair_ratio }` — splitter moved
- `LayoutChanged` — reserved for future use

Layout mutations from tab/split messages are handled inside the widget before your callback runs, so the dock stays consistent even if the app only forwards messages.

## Keyboard navigation

The crate does not subscribe to keys itself. Use `adjacent_pane` with bounds from the last draw pass:

```rust
use iced_dock::{adjacent_pane, pane_bounds_map, Direction, DockMessage};

if let Some(pane) = session.focused_pane() {
    let bounds = pane_bounds_map(&session.state().borrow().pane_bounds);
    if let Some(next) = adjacent_pane(pane, Direction::Right, &bounds) {
        session.apply_message(DockMessage::PaneFocused {
            pane: next,
            panel: None,
        });
    }
}
```

See `examples/minimal.rs` for a `keyboard::listen` subscription with `Ctrl+Arrow`.

## Styling

`DockStyle` groups:

- `background` — gaps between panes
- `window` — pane frame, border, focused border accent
- `tab_bar` / `tab` — tab strip and labels
- `splitter` — resize handles and gaps
- `drop_overlay` — drag target highlight

Use `constant(my_style)` for a fixed style, or `DockStyle::modern_dark()` as a starting point.

## Serialization

Enable the `serde` feature to persist layouts:

```toml
iced_dock = { version = "0.1", features = ["serde"] }
```

- **`LayoutTree`** — save default/workspace templates
- **`Layout`** — save full runtime state after user splits and tab changes

## Project structure

```
src/
  builder/     Declarative LayoutTree, DockSession, compile to Layout
  model/       Layout graph (Panel, Pane, ProportionalGroup)
  factory/     Layout mutations
  manager/     Drag validation and drop execution
  spatial/     adjacent_pane for keyboard focus
  style/       DockStyle and theme helpers
  widget/      Dock, TabDock, TabStrip, SplitContainer
examples/
  minimal.rs   Full IDE-style demo
```

## License

MIT
