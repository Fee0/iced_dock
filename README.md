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
- **Click to focus** — clicking a pane's content area focuses that pane and emits `DockEvent::PaneFocused`.
- **Tab click** — selects the tab and focuses its pane.
- **Keyboard navigation** — `DockSession::focus_adjacent` or the lower-level `adjacent_pane` helper (gap-tolerant); wire to `Ctrl+Arrow` or similar in your app.
- **Pane targets** — open new panels into the focused pane (`PaneTarget::Active`), a named pane, or the first pane in tree order.

### Styling

- **Generic `Theme`** — `Dock<Message, Theme, Renderer>` is generic over `Theme: Catalog` (same pattern as `pane_grid`). Default type parameters (`Theme = iced::Theme`, `Renderer = iced::Renderer`) keep existing code unchanged.
- **Palette default** — built-in dock chrome follows the active iced theme via `style::default` / [`Catalog`](src/style.rs).
- **Optional IDE presets** — `style::preset::modern_dark()` / `modern_light()` for VS Code–inspired chrome (explicit opt-in via `.style(...)`).
- **Per-pane styling** — the content closure can return `PaneContent` with a per-pane style override, giving individual panes different tab bars, borders, and chrome.
- **Customizable chrome** — pane borders (including focused accent), tab colors, splitter handles, drop overlays, tab bar metrics, close buttons.
- **Builder overrides** — `min_pane_width`, `min_pane_height`, tab bar scrollbar visibility and hide delay; `.style(...)`, `.class(...)`.

### Integration

- **iced widget** — `dock()` builder returns a `Dock` widget that plugs into any iced `Element` tree.
- **Observation events** — [`DockEvent`] uses string panel/pane ids (no slotmap handles in app code).
- **Widget-owned mutations** — user input is applied inside the dock before your `on_event` callback; use `update` for side effects only.
- **Content mapping** — closure `ContentKey → Element`; store shared app state in `Rc` if needed.

### Runtime API

- **`DockSession`** — open, focus, and close panels by string id; shares one `DockWidgetState` with the widget.
- **`DockWidgetState`** — layout graph, string index, drag session, focused pane, pane bounds.
- **`iced_dock::unstable`** — `Factory`, `DockManager`, `dispatch_action`, `build_tree`, and other low-level helpers.

### Persistence (optional)

- **`serde` feature** — serialize/deserialize declarative `LayoutTree` and runtime `Layout`.
- Prefer **`LayoutTree`** for workspace templates. Runtime **`Layout`** is for session restore in the same app version; slotmap `NodeId` values are not stable semantic handles across refactors.

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
    dock, horizontal, panel, tabs, ContentKey, DockEvent, DockSession, LayoutTree,
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
    Dock(DockEvent),
}

fn view(app: &App) -> Element<'_, Message> {
    dock()
        .state(app.dock.state())
        .on_event(Message::Dock)
        .content(|key| panel_content(key))
        .build()
        .into()
}

fn panel_content(key: ContentKey) -> Element<'static, Message> {
    text(format!("Panel {}", key.0)).into()
}

fn update(_app: &mut App, message: Message) {
    if let Message::Dock(_event) = message {
        // Layout already updated by the widget — log, sync title bar, etc.
    }
}
```

Run the full demo (multi-split IDE layout, focus border, `Ctrl+Arrow` navigation):

```bash
cargo run --example minimal
```

Optional prelude:

```rust
use iced_dock::prelude::*;
```

## Core concepts

| Concept | Description |
|---------|-------------|
| **Panel** | A tab: string id, title, and `ContentKey` that maps to your UI. |
| **Pane** | A tab group showing one active panel; optional name for `PaneTarget::Named`. |
| **Active tab** | Which panel is visible inside a pane. |
| **Focused pane** | Which pane last received attention globally. |
| **ContentKey** | Opaque `u32` you use to build the correct `Element` for a panel. |
| **LayoutTree** | Declarative spec compiled once into runtime `Layout`. |
| **DockEvent** | App-facing notification (string ids). |
| **DockAction** | Internal command; use `DockSession::dispatch` for programmatic control. |

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

Use `DockSession::from_tree(tree)` or `iced_dock::unstable::build_tree(&tree)` for standalone compilation.

## Theming

By default, the dock uses [`style::default`](src/style.rs) (colors from `theme.extended_palette()`, layout metrics from the built-in dark preset).

| Goal | Setup |
|------|--------|
| Match iced Light/Dark/Custom themes | `dock().build()` with no `.style(...)` |
| VS Code–style chrome | `.style(iced_dock::preset::modern_dark())` or `preset::modern_light()` |
| Fixed custom chrome | `.style(iced_dock::constant(my_style))` or `.style(\|t\| { ... })` |
| Per-pane chrome | Return `PaneContent::new(element).style(\|t\| custom_dock_style)` from the content closure |
| Custom `Theme` type | `Dock<Message, MyTheme>` — implement `iced_dock::Catalog` for `MyTheme` |
| Panel interiors | Style your `content` closure (containers, text); not part of dock chrome |

### Per-pane styling

The content closure can return a `PaneContent` to override the dock-level style for individual panes:

```rust
use iced_dock::{PaneContent, DockStyle, ContentKey};

fn panel_content(key: ContentKey) -> PaneContent<'static, Message> {
    let element = iced::widget::text(format!("Panel {}", key.0)).into();
    if key.0 == 10 {
        PaneContent::new(element).style(|_theme| DockStyle::modern_dark())
    } else {
        PaneContent::new(element)
    }
}
```

Per-pane overrides affect tab bar, tab labels, window chrome, and drop overlays. Splitter styling is not per-pane (splitters sit between panes and use the dock-level style).

### Custom Theme types

The dock widget is generic: `Dock<Message, Theme = iced::Theme, Renderer = iced::Renderer>`. To use a custom theme, implement `iced_dock::Catalog` for your type:

```rust
impl iced_dock::Catalog for MyTheme {
    type Class<'a> = iced_dock::StyleFn<'a, Self>;
    fn default() -> Self::Class<'static> { Box::new(|_| DockStyle::default()) }
    fn style(&self, class: &Self::Class<'_>) -> DockStyle { class(self) }
}
```

`DockStyle::from_theme` is deprecated; it now resolves to the palette default, not the IDE presets.

## Dock widget builder

```rust
use iced_dock::preset;

dock()
    .state(session.state())
    .on_event(Message::Dock)              // map DockEvent → app Message
    .content(|key| view_panel(key))       // Fn + 'static (Rc-friendly)
    .style(preset::modern_dark())         // optional; omit for palette default
    .min_pane_width(200.0)
    .min_pane_height(120.0)
    .tab_bar_show_scrollbar(false)
    .tab_bar_scrollbar_hide_delay(Duration::from_secs(1))
    .build()
```

## DockSession

| Method | Purpose |
|--------|---------|
| `from_tree` | Build session from `LayoutTree` (focuses first pane) |
| `from_tree_with_focus` | Build with [`InitialFocus`] |
| `from_built` | Build from `unstable::BuiltLayout` + optional focused pane |
| `state()` | Shared `Rc<RefCell<DockWidgetState>>` for the widget |
| `dispatch(action)` | Apply [`DockAction`] programmatically (not for widget events) |
| `open_panel(target, def)` | Add and activate a panel |
| `select_panel(id)` | Activate tab and focus its pane |
| `focus_pane(node_id)` | Focus pane without changing active tab |
| `focus_adjacent(direction)` | Move pane focus (needs one draw pass first) |
| `cycle_panel(cycle)` | Next/prev tab in focused pane |
| `clear_focus()` | Clear global pane focus |
| `close_panel(id)` | Close tab and collapse empty panes |
| `focused_pane()` | Current focused pane `NodeId` (advanced) |
| `active_panel()` | Active tab id in the **focused** pane |
| `active_panel_in_pane(pane)` | Active tab in any pane |
| `panel_ids()` | All registered panel ids |

`PaneTarget`: `Active`, `Named("pane_name".into())`, `First`.

**Focus vs active tab:** `select_panel` changes both; `focus_pane` changes only the focused border; `active_panel_in_pane` reads any pane.

## Events

[`DockEvent`] variants (string ids):

- `TabSelected { pane, panel }` — tab clicked
- `TabClosed { panel }` — close button
- `PaneFocused { pane, panel }` — content click or programmatic focus
- `SplitResized { splitter_index, pair_ratio }` — splitter moved
- `DragStarted` / `DragMoved` / `DragEnded` / `DragCancelled` — tab drag lifecycle
- `LayoutChanged` — structural layout change

The widget applies mutations before `on_event` runs. Do **not** call `dispatch` for widget-originated input.

## Keyboard navigation

The crate does not subscribe to keys itself. [`DockSession::focus_adjacent`] uses pane bounds from the last draw pass — run the dock once (or wait for the first frame) before calling:

```rust
session.focus_adjacent(Direction::Right);
```

For custom logic, use [`adjacent_pane`] with [`pane_bounds_map`] on `session.state().borrow().pane_bounds`.

See `examples/minimal.rs` for `keyboard::listen` with `Ctrl+Arrow`.

## Advanced API

Import `iced_dock::unstable` for `Factory`, `DockManager`, `dispatch_action`, `build_tree`, `DockAction`, `TabAction`, and compile helpers.

Use `iced_dock::model` for `Layout`, `NodeId`, and graph types when persisting or introspecting the arena.

## Serialization

```toml
iced_dock = { version = "0.1", features = ["serde"] }
```

- **`LayoutTree`** — workspace templates (recommended for defaults)
- **`Layout`** — full runtime state after user edits (same app version)

## Project structure

```
src/
  builder/     LayoutTree, DockSession, compile
  model/       Layout graph (Panel, Pane, NodeId)
  unstable/    Factory, DockManager, build_tree, dispatch_action
  spatial/     adjacent_pane
  style/       DockStyle
  widget/      Dock, DockEvent, DockAction
  prelude.rs   Common imports
examples/
  minimal.rs   IDE-style demo
```

## License

MIT
