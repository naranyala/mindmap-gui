# Architecture

How `mindmap-gui` is put together. Read this before touching `src/`.

The design goal is a **thin shell over a pure model**: the data model knows
nothing about `egui`, the canvas knows nothing about file dialogs, and the app
shell owns orchestration. Every mutation flows one direction.

```
            user input
                │
   ┌────────────▼────────────┐
   │  app.rs  (shell)        │  menus, shortcuts, dialogs, file ops
   │  - owns MindMapApp      │
   │  - decides *when*       │
   └───────┬─────────┬───────┘
           │         │
   ┌───────▼───┐ ┌───▼────────┐
   │ model.rs  │ │ canvas.rs  │
   │ (pure)    │ │ (egui)     │
   │ decides   │ │ decides    │
   │ *what*    │ │ *how it    │
   │           │ │  looks*    │
   └───────────┘ └────────────┘
        │               │
        └───────┬───────┘
                ▼
           JSON on disk
```

---

## The four modules

| Module | Responsibility | Knows about |
| --- | --- | --- |
| `main.rs` | eframe bootstrap, viewport, CLI arg | `eframe`, `MindMapApp` |
| `app.rs` | Shell: menus, status bar, keyboard shortcuts, file dialogs, unsaved-changes guard | everything |
| `canvas.rs` | Rendering + pointer/keyboard interaction on the map | `egui`, `model` |
| `model.rs` | `MindMap` / `Node`, tree operations, JSON (de)serialization | `serde`, `egui::{Pos2,Rect,Vec2}` |

`model.rs` is the only file with unit tests (9 of them). It is deliberately
kept free of UI logic so the tree rules are testable in isolation.

---

## `main.rs` — bootstrap

```rust
eframe::run_native("mindmap-gui", options, Box::new(move |cc| {
    Ok(Box::new(MindMapApp::new(cc, initial_path)))
}))
```

- Viewport defaults to **1200×800**, minimum **720×520**, title `"Mindmap"`.
- The first CLI argument, if present, is opened at startup (`cargo run -- map.json`).
- The backend is **glow** (OpenGL) with both `x11` and `wayland` features enabled
  so it runs on any Linux desktop without a recompile.

Note the `eframe` 0.37 signature: the app implements
`fn ui(&mut self, ui: &mut Ui, frame: &mut Frame)` — **not** the older
`fn update(&mut self, ctx, frame)`. Panels are therefore drawn against the
root `Ui`, and you get a `Context` from `ui.ctx().clone()` when you need one.

---

## `app.rs` — the shell

`MindMapApp` owns all application state:

```rust
pub struct MindMapApp {
    map: MindMap,                 // the document
    canvas: Canvas,               // camera + scene
    selected: Option<NodeId>,     // one node at a time (v0.1)
    editing: Option<Editing>,     // in-flight text edit, if any
    dirty: bool,                  // unsaved changes?
    file_path: Option<PathBuf>,   // where it lives on disk
    status: String,               // status-bar text
    show_help: bool,              // shortcuts window open?
    pending: Option<PendingOp>,   // deferred action awaiting save-guard
}
```

### Frame layout

`run_ui` builds the frame in a fixed order:

1. `handle_close_request` — intercept OS close if dirty (`CancelClose` + modal).
2. `Panel::top("main_menu")` — `MenuBar` with File / View / Help.
3. `Panel::bottom("status_bar")` — status text left, zoom/node-count/filename right.
4. `shortcuts` — keyboard handling, skipped while editing or a modal is up.
5. `help_window` + `confirm_dialog` — floating windows.
6. `CentralPanel` — hands off to `Canvas::show`.

### The save-guard flow

Every destructive action (`New`, `Open`, `Quit`) funnels through a
`PendingOp`:

```
request_new() ──► dirty? ──no──► do_new()
                    │
                   yes
                    ▼
              pending = Some(New)  ──► confirm_dialog() shows a Modal
                                            │
                        ┌───────────────────┼───────────────────┐
                     Save               Don't save            Cancel
                        │                   │                   │
                     save()           pending = None        pending = None
                        │             do_new()              (do nothing)
                   !dirty? ──yes──► run_pending()
```

`run_pending` is the single place that actually executes the deferred action,
so the guard can never be bypassed by accident.

`flush_edit` is called before any file operation; it writes the in-progress
inline edit back into the map so nothing in the text buffer is ever lost.

### Keyboard dispatch

`shortcuts` is a flat `if/else` chain over `ctx.input(|i| i.key_pressed(..))`.
It is guarded at the top by `pending.is_some() || editing.is_some()` so a
modal or a text edit always wins over global chords. This is intentionally
simple; the roadmap (task T4.1) replaces it with a named-action keymap table
so shortcuts become rebindable and searchable by a command palette.

---

## `canvas.rs` — rendering & interaction

### Why `egui::Scene`

The canvas is an `egui::Scene`, not hand-rolled pan/zoom math. `Scene` gives us:

- **Free panning** — scroll and drag-the-background just work, with momentum.
- **Zoom** — `Ctrl+scroll` zooms around the cursor; `zoom_range` is clamped to
  `0.1..=4.0`.
- **Layer transforms** — hit-testing, `Response::contains_pointer`, and text
  cursor placement are all correct inside the zoomed/panned coordinate space,
  which is the hard part of a canvas and easy to get subtly wrong by hand.
- **Clipping** — off-screen content is culled for free.

The visible rectangle is `scene_rect: Rect` (in *scene* coordinates, not
screen). `zoom` is derived each frame from the ratio of the panel size to
`scene_rect`'s size. `zoom_by(factor)` shrinks/grows `scene_rect` around its
center; `center_on(rect)` sets it to a content bounding box to "fit".

### The render pass

`show` runs in three phases inside the `Scene` closure:

1. **Measure** — for every node, lay out its text with `layout_no_wrap`, add
   padding, clamp to `MIN_NODE_WIDTH/HEIGHT`. Result: `sizes: HashMap<NodeId, Vec2>`.
   This runs for *every node every frame* — the known performance debt (task T1.5
   caches it).
2. **Paint edges** — for each node with a parent, draw a cubic bezier between
   the two node rects. Anchors are computed by `edge_anchor`, which finds where
   the line from parent-center to child-center crosses each rect's border, so
   edges touch the boxes instead of starting at their centers.
3. **Paint/interact nodes** — for each node, either an inline `TextEdit` (if it
   is the one being edited) or a painted rounded-rect plus an `ui.interact` hit
   region.

Interaction results (drags, clicks, edit-commit) are **collected during the
loop and applied after it**, so the `&mut MindMap` is not borrowed mutably
while we are iterating `&map.nodes`.

### The edit lifecycle

```rust
pub struct Editing { id: NodeId, text: String, fresh: bool }
```

- `fresh: true` means "this is the first frame" — we request focus and set the
  cursor to select all text so typing immediately replaces the placeholder.
- Commit on `Enter` **or** focus loss **or** click-outside.
- Revert on `Escape`.
- `finish: Option<(NodeId, Option<String>)>` is the deferred result; `None`
  inner means "revert", `Some(text)` means "commit".

---

## `model.rs` — the document

### Tree representation

```rust
struct MindMap { root: NodeId, nodes: Vec<Node>, next_id: u64 }
struct Node   { id, parent: Option<NodeId>, children: Vec<NodeId>, pos, text }
```

Edges are stored **twice**: each node holds a `parent` pointer *and* the parent
holds ordered `children`. This makes both directions O(1)-ish to walk, at the
cost of keeping them consistent — which `restructure` enforces on load.

`nodes` is a `Vec` and lookups are linear scans (`get`/`get_mut`). That is fine
for hundreds of nodes and is the first structural refactor planned (task T1.1
switches to `HashMap<NodeId, Node>` + a stable order vector).

### Placement heuristics

New nodes are placed with constants, not a layout engine:

| Constant | Value | Meaning |
| --- | --- | --- |
| `CHILD_DX` | 280 | horizontal distance from parent for a new child |
| `CHILD_GAP` | 76 | vertical spacing between successive children |
| `SIBLING_GAP` | 84 | vertical spacing for a new sibling |

This produces a readable left-to-right tree by default but can overlap once
you drag things around — the reason a real smart-placement pass (T1.4) and
auto-layout engines (T2.10) are on the roadmap.

### Repair on load

`restructure` is called by `load_from_str` and fixes up anything a hand-edited
or corrupted file might contain:

1. Drop child IDs that don't correspond to a real node (or self-references).
2. BFS from `root`, keep only reachable nodes.
3. Rebuild every `parent` pointer from the `children` lists (children are
   treated as the source of truth).

If the root itself is missing, `restructure` returns `false` and the load
fails with a clear error rather than opening a broken map.

---

## Data flow: a keystroke to a saved file

Taking "press Tab to add a child" end to end:

```
1. app.rs::shortcuts()          sees Key::Tab, no modal/editing active
2. app.rs::add_child()          target = selected ?? root
3. model.rs::add_child()        computes pos, calls insert_node()
4. model.rs::insert_node()      allocates next_id, pushes Node, splices into parent.children
5. app.rs::add_child()          sets dirty=true, selected=new id
6. app.rs::add_child()          calls start_editing() → Editing{fresh:true}
7. canvas.rs::show()            next frame: measures node, paints TextEdit,
                                requests focus, selects all text
8. (user types, presses Enter)
9. canvas.rs::show()            finish = Some((id, Some(text)))
10. canvas.rs::show()           map.set_text(id, text) → dirty=true, editing=None
11. (user presses Ctrl+S)
12. app.rs::save()              flush_edit(), then do_save(path)
13. model.rs::save_file()       save_to_string() → serde_json → fs::write
```

---

## Threading & I/O

**Today everything is synchronous on the UI thread.** File dialogs block (that
is what `rfd` does natively, and it is acceptable for a modal action); save/load
of a JSON file is a single `fs::read`/`fs::write`.

The roadmap moves the expensive work off-thread:

| Task | Work | Plan |
| --- | --- | --- |
| T1.6 | autosave | background thread + channel, ≤5 s debounce |
| T1.5 | text layout cache | measure-on-change, not per-frame |
| T2.10 | auto-layout for 1 000 nodes | worker thread, results applied next frame |
| T3.7 | PNG/SVG export | worker + progress dialog |

The rule from the pyramid (principle **P2**): *no synchronous I/O on the UI
thread once the operation can exceed ~5 ms.* Today's operations are all
sub-millisecond on realistic maps, so the shortcut is safe — but it is a
shortcut, not a design.

---

## Extension points

When adding a feature, this is where it goes:

| Kind of feature | Put it in | Notes |
| --- | --- | --- |
| New tree operation | `model.rs` + a unit test | must be undoable (see T1.3) |
| New keyboard action | `app.rs::shortcuts` (until T4.1) | keep the guard order |
| New menu item | `app.rs::menus` | use the `menu_item` helper |
| New node visual | `canvas.rs` render loop | keep measurement & paint in sync |
| New file operation | `model.rs` `save_*`/`load_*` | bump `FILE_VERSION` if schema changes |
| New panel | `app.rs::run_ui` | `Panel::top/bottom`, then `CentralPanel` last |

The invariant to preserve: **`model.rs` must not learn about `egui` widgets.**
`Pos2`/`Rect`/`Vec2` are fine (they are pure math types re-exported by `egui`);
`Response`, `Ui`, `Painter` are not.
