# Pyramid of Intents — Mindmap Desktop App

**Working title:** Mindmap · **crate:** `mindmap-gui` · **stack:** Rust + egui/eframe

## How to read this document

The pyramid is read **top-down**: every level exists only to serve the level
above it. Two traceability rules gate all work on this project:

1. **Upward trace** — a feature that cannot be tied to a user intent in **L3**
   does not ship. No "nice to haves" without an owner.
2. **Downward trace** — an intent in **L3** with no capability in **L5** is
   vapor. Either build it or delete the intent.

Task IDs in [`TODOS.md`](./TODOS.md) reference intent IDs (`I#`) from **L3**
directly, so every checkbox in the todo list can be traced to a reason to
exist.

```
            ▲  L1  Purpose          why the app exists at all
           ╱ ╲
          ╱   ╲  L2  Principles     non-negotiable constraints on how
         ╱     ╲
        ╱       ╲  L3  User Intents what people actually want, ranked
       ╱         ╲
      ╱           ╲  L4  Guarantees  measurable promises per intent
     ╱             ╲
    ╱               ╲  L5  Capabilities  concrete features, mapped up
   ╱                 ╲
  ╱___________________╲  L6  Foundation   architecture, data, budgets
```

---

## L1 — Purpose (apex)

> **Turn tangled thought into legible structure, without losing the thought.**

Everything below is judged against this sentence. A mindmap tool is not a
drawing tool and not a document editor — it is a *thinking* tool. The value
is the shape of the idea, not the pixels of the node.

**Anti-goals** (things that would betray the purpose):

- Becoming a general whiteboard/canvas app (infinite distractions, no focus).
- Requiring an account, a sync service, or a subscription to save a file.
- Trading responsiveness for visual polish.

---

## L2 — Principles

Non-negotiable constraints. When two intents conflict, these decide.

| ID  | Principle | Concretely means |
|-----|-----------|------------------|
| **P1** | **Local-first & private** | Files live on the user's disk in a readable format. No telemetry, no account, no network required for any core feature. |
| **P2** | **Fluid — never block the canvas** | No synchronous file/IO work on the UI thread. No per-frame work proportional to map size without caching. Input latency budget: **< 16 ms** worst case. |
| **P3** | **Forgiving — undo everything, autosave always** | Every mutation is undoable. Work is persisted without the user having to remember. Closing with unsaved changes is guarded; crashing loses at most the autosave interval. |
| **P4** | **Keyboard-first, mouse-complete** | Every action reachable from the keyboard without chords that strain the hand. Every action also reachable by pointer, because some people only have a trackpad. |
| **P5** | **Simple core, progressive disclosure** | The first screen is a canvas and a root node. Advanced power (styles, layout engines, exporters) hides behind a command palette and context menus, not permanent chrome. |
| **P6** | **Honest file format** | Human-readable, versioned, forward-migratable. A user who loses the app can still read their data in a text editor. Unknown fields round-trip, never dropped. |

---

## L3 — User Intents

Ranked. **P0** is the product; without it there is nothing. **P1** is what makes
people stay. **P2** is what makes people recommend it. **P3** is speculative and
explicitly fenced.

### P0 — the product

| ID | Intent | The job to be done | Success looks like |
|----|--------|--------------------|--------------------|
| **I1** | **Capture** | "I have a thought *right now* and it will evaporate." | From cold app start to a labelled node on screen in **< 3 s**, all keyboard. Creating a child never requires touching the mouse. |
| **I2** | **Organize** | "These ideas are related but I can't see how yet." | Regroup, reorder, fold, and re-lay-out ideas spatially until the *shape* of the argument is visible. Drag a subtree somewhere else and the structure follows. |
| **I3** | **Persist** | "I must not lose this." | Save is one keystroke; autosave is invisible; the app reopens yesterday's map exactly as left — including zoom, pan, selection, and fold state. |

### P1 — why people stay

| ID | Intent | The job to be done | Success looks like |
|----|--------|--------------------|--------------------|
| **I4** | **Navigate** | "This map has 400 nodes and I know the idea is in here somewhere." | Jump to any node by search in **< 1 s**; never lose spatial context of where the match sits in the whole. |
| **I5** | **Refine** | "I need this to communicate, not just exist." | Mark priority/progress/status, colour branches, attach notes and links — without leaving the canvas flow. |
| **I6** | **Express** | "I need to hand this to someone who doesn't have the app." | Export to image, outline text, and print-ready PDF. What they receive is faithful to what I see. |

### P2 — what makes people recommend it

| ID | Intent | The job to be done | Success looks like |
|----|--------|--------------------|--------------------|
| **I7** | **Present** | "I want to walk a room through my thinking." | Fullscreen step-through that animates between nodes in reading order, no editing affordances visible. |
| **I8** | **Extend** | "My data should flow to and from my other tools." | Import/export Markdown, OPML, Freeplane `.mm`. Open a `.md` outline as a map and vice versa. |

### P3 — fenced / out of near-term scope

| ID | Intent | Why it's fenced |
|----|--------|-----------------|
| **I9** | **Collaborate** | Real-time multiplayer needs a server, conflict resolution, and an account model — all three violate **P1**. Revisit only as an explicit *opt-in* overlay that never becomes a requirement. |
| **I10** | **Script** | A plugin runtime (Rhai/Lua) is powerful but freezes the data model and the security story. Deferred until **L5** is stable enough to have a real API surface. |

---

## L4 — System Guarantees

The measurable promises that make each intent true. These are the acceptance
criteria the whole project is tested against.

| Intent | Guarantee | Budget / metric |
|--------|-----------|-----------------|
| **I1** | Cold start to interactive canvas | < 500 ms on a mid-range laptop (release build) |
| **I1** | Keystroke → node appears | 1 frame; no modal, no focus trap |
| **I1** | New child placement never overlaps an existing node | Deterministic; tested across 1 000-node maps |
| **I2** | Undo is a single keystroke, arbitrary depth | ≥ 200 steps retained; coalesced drags = 1 step |
| **I2** | Auto-layout completes for 1 000 nodes | < 50 ms, on a worker or chunked |
| **I3** | Autosave interval | ≤ 5 s after last edit, to a recovery file |
| **I3** | Crash loses at most | one autosave interval |
| **I3** | File format round-trips unknown fields | byte-identical on re-save (modulo formatting) |
| **I4** | Search across 1 000 nodes | < 100 ms, incremental |
| **I4** | Minimap never drops frame budget | culled + LOD'd; contributes < 2 ms |
| **I5** | Style change is undoable in one step | same as any mutation |
| **I6** | PNG export of a 1 000-node map | < 2 s, correct at 2× and 4× scale |
| **I6** | Outline export is valid, re-importable | round-trip test in CI |
| **I7** | Presentation advances without reflow | precomputed camera targets |
| **I8** | Every importer is lossless *or* says what it drops | never silent data loss |

---

## L5 — Capabilities

Concrete features, each traced to the intents it serves and to its status.

Status: **shipped** (in the code today) · **planned** (in `TODOS.md`) ·
**fenced** (L3 P3).

### Capture & edit

| # | Capability | Serves | Status | Notes |
|---|-----------|--------|--------|-------|
| C1 | Create child / sibling node | I1 | **shipped** | `Tab` / `Enter` |
| C2 | Inline text editing, select-on-create | I1 | **shipped** | `F2` / double-click |
| C3 | Delete subtree | I2 | **shipped** | root protected |
| C4 | Undo / redo with coalescing | I1 I2 I3 | planned | command pattern |
| C5 | Multi-select (shift-click, marquee) | I2 | planned | |
| C6 | Copy / cut / paste / duplicate | I1 I2 | planned | cross-map paste |
| C7 | Drag to re-parent (drop on node) | I2 | planned | reparent + optional move |
| C8 | Arrow-key spatial navigation | I2 I4 | planned | nearest node in direction |
| C9 | Fold / expand subtree | I2 I4 | planned | state persisted |
| C10 | Note / detail field per node (markdown) | I5 | planned | |

### Arrange & view

| # | Capability | Serves | Status | Notes |
|---|-----------|--------|--------|-------|
| C11 | Free-form drag positioning | I2 | **shipped** | current default |
| C12 | Pan / zoom canvas, fit-to-map | I2 I4 | **shipped** | `Scene`, 0.1–4× |
| C13 | Auto-layout engines (tree, mindmap, radial, org) | I2 | planned | opt-in per map or per branch |
| C14 | Overlap-free smart placement for new nodes | I1 I2 | planned | fixes naive `CHILD_DX` offset |
| C15 | Minimap with viewport box | I4 | planned | |
| C16 | Outline / list side panel (two-way sync) | I4 | planned | |
| C17 | Command palette (`Ctrl+K`) | P4 P5 | planned | every command, fuzzy |
| C18 | Context menu on node and canvas | P4 P5 | planned | |
| C19 | Themes: light / dark / system, accent | I5 | planned | |
| C20 | Presentation mode | I7 | planned | |
| C21 | Customizable keyboard shortcuts | P4 | planned | keymap table, not hard-coded |

### Style & annotate

| # | Capability | Serves | Status | Notes |
|---|-----------|--------|--------|-------|
| C22 | Per-node colour, shape, border | I5 | planned | |
| C23 | Branch / theme colour inheritance | I5 | planned | |
| C24 | Icons & markers (priority, progress, flag) | I5 | planned | |
| C25 | Hyperlinks on nodes | I5 I8 | planned | |
| C26 | Search / find, with highlight + jump | I4 | planned | |

### Files, export, import

| # | Capability | Serves | Status | Notes |
|---|-----------|--------|--------|-------|
| C27 | JSON save / open, versioned format | I3 | **shipped** | v1, migration-aware |
| C28 | Unsaved-changes guard on new/open/quit | I3 | **shipped** | modal |
| C29 | Open file via CLI argument | I3 | **shipped** | `cargo run -- file.json` |
| C30 | Autosave + crash recovery | I3 | planned | shadow file |
| C31 | Session restore (last map, view, selection) | I3 | planned | |
| C32 | Recent files list | I3 | planned | |
| C33 | Export PNG / SVG | I6 | planned | |
| C34 | Export PDF (print vector) | I6 | planned | |
| C35 | Export Markdown / OPML / plain outline | I6 I8 | planned | |
| C36 | Import Markdown / OPML / Freeplane `.mm` | I8 | planned | |
| C37 | File-format v2 (notes, style, fold, view) | I3 I5 | planned | schema below |

### Fenced (explicitly not now)

| # | Capability | Intent | Status |
|---|-----------|--------|--------|
| C38 | Real-time collaboration | I9 | **fenced** |
| C39 | Cloud sync / account | I9 | **fenced** — violates P1 |
| C40 | Plugin / scripting runtime | I10 | **fenced** |
| C41 | Mobile / web build | — | **fenced** — desktop is the product |

---

## L6 — Foundation

### Current architecture (as built)

```
src/
  main.rs     eframe bootstrap, NativeOptions, CLI arg → MindMapApp::new
  app.rs      App shell: Panel top/bottom, CentralPanel, menus, dialogs,
              shortcuts, file ops, unsaved-changes Modal
  canvas.rs   egui::Scene pan/zoom, node paint, hit-test, inline TextEdit
  model.rs    MindMap / Node / NodeId, tree ops, bounds, restructure,
              JSON FileDoc v1, + unit tests
```

Key facts a new contributor needs:

- **eframe 0.37** implements `App::ui(&mut self, ui, frame)` — *not* the older
  `update(ctx)`. Panels are `egui::Panel::top/bottom`, shown against the root
  `Ui`.
- **Canvas is an `egui::Scene`**, so pan/zoom hit-testing and layer transforms
  are handled by egui; we paint in scene coordinates and register node rects
  with `ui.interact`.
- **Node measurement** (`layout_no_wrap`) runs for every node every frame.
  This is the first thing to cache when maps grow.
- **`model.rs` looks nodes up by linear scan** (`Vec<Node>` + `iter().find`).
  Fine for hundreds of nodes; the first structural refactor for thousands.

### Target architecture

Split by responsibility; the shell stays thin, the model stays pure (no `egui`
types leaking into it), the UI never mutates state except through commands.

```
src/
  main.rs
  app.rs                  shell / orchestration only
  model/
    mod.rs                MindMap, Node, NodeId  (pure data + queries)
    command.rs            Command enum + apply/invert  (undo backbone)
    history.rs            undo/redo stacks, coalescing policy
    layout.rs             tree / radial / org layout engines
    fold.rs               fold state + visibility computation
    style.rs              NodeStyle, palettes, theme resolution
    query.rs              search, spatial index, nearest-node
    format.rs             FileDoc v2, version gate, migrations, round-trip
  canvas/
    mod.rs                Scene host, camera, render loop
    node_widget.rs        node paint + hit rect
    edge.rs               bezier routing, anchors
    minimap.rs
    interaction.rs        pointer/keyboard state machine
  ui/
    outline.rs            list panel (two-way sync)
    palette.rs            command palette
    inspector.rs          style / details panel
    context_menu.rs
    dialogs.rs            confirm, export, import, about
    status.rs
    theme.rs              egui Visuals wiring from model::style
  io/
    autosave.rs           shadow file, recovery prompt
    session.rs            recent files, window + view state
    export/{png,svg,pdf,markdown,opml}.rs
    import/{markdown,opml,freemind}.rs
  input/
    keymap.rs             named actions → chords, user-rebindable
```

### Data model — target shape (format v2)

Extends today's v1 in a backward-compatible way. Every new field has a serde
default so v1 files load unchanged.

```rust
struct MindMap {
    root: NodeId,
    nodes: HashMap<NodeId, Node>,   // replaces Vec + O(n) scan
    order: Vec<NodeId>,             // stable paint/serialize order
    next_id: u64,
}

struct Node {
    id: NodeId,
    parent: Option<NodeId>,
    children: Vec<NodeId>,
    pos: Pos2,
    text: String,                   // title
    #[serde(default)] note: String, // markdown detail      (C10)
    #[serde(default)] style: NodeStyle,                     (C22)
    #[serde(default)] folded: bool,                         (C9)
    #[serde(default)] link: Option<String>,                 (C25)
    #[serde(default)] icon: Option<Icon>,                   (C24)
}

struct FileDoc {
    app: String,
    version: u32,                   // 1 → 2 when the above lands
    map: MindMap,
    #[serde(default)] view: ViewportState,  // camera, selection  (C31)
    #[serde(default)] meta: MapMeta,        // title, created, modified
}
```

`ViewportState { center: Pos2, zoom: f32, selected: Option<NodeId> }` makes
**I3**'s "reopens exactly as left" concrete.

### Undo/redo design (C4)

Command pattern — every mutation is a `Command` that can apply *and* invert.

```rust
enum Command {
    AddNode     { id, parent, after, text, pos },
    DeleteSubtree { snapshot: Vec<Node> },   // capture whole subtree
    SetText     { id, old, new },
    MoveNode    { id, old_pos, new_pos },
    Reparent    { id, old_parent, old_index, new_parent, new_index },
    SetStyle    { id, old: NodeStyle, new: NodeStyle },
    SetFolded   { id, folded },
    Batch(Vec<Command>),                     // multi-select ops
}
```

Coalescing policy (so one drag = one undo step): merge consecutive
`MoveNode`/`SetText` on the same `id` within **800 ms** into one entry.
`Batch` wraps multi-select mutations so they undo atomically.

### Performance budgets & strategy

| Risk | Strategy |
|------|----------|
| O(n) node lookup (`get`/`get_mut`) | `HashMap<NodeId, Node>` + `order: Vec<NodeId>` |
| Galley measured every frame for every node | cache keyed by `(text, font, zoom_bucket)`; invalidate on edit only |
| Off-screen nodes still painted | viewport cull against `scene_rect` inflated by margin |
| Tiny nodes unreadable, expensive to shape | LOD: below zoom threshold draw filled rect, no text/edges |
| Auto-layout blocks UI | chunked or `std::thread` + channel; never on the frame path |
| Save blocks on big map | serialize on a worker; UI keeps editing against a snapshot |

### Tech stack & dependencies

| Concern | Choice | Why |
|---------|--------|-----|
| GUI | `eframe` 0.37 (glow backend) | matches the egui already in use; lighter than wgpu |
| Canvas | `egui::Scene` | free pan/zoom + layer transform hit-testing |
| File dialogs | `rfd` | portal/GTK dialogs on Linux, native on macOS/Windows |
| Serialization | `serde` + `serde_json` | already in use; format is JSON per **P6** |
| Images (export) | `resvg` + `tiny-skia` | SVG → raster without a GPU dependency |
| PDF | `printpdf` or `typst` embed | vector output, not screenshot-of-bitmap |
| Async (autosave, export) | `std::thread` + `mpsc` | no runtime dependency; enough for this app |

### Verification standard

Nothing is "done" until all four hold:

1. `cargo build --release` clean, no warnings.
2. `cargo clippy --all-targets -- -D warnings` clean.
3. `cargo test` green — model/format/layout get real unit tests, not smoke.
4. The acceptance criteria in `TODOS.md` for that task are manually exercised
   on at least one map larger than 300 nodes.

---

## Scope boundary (the honest summary)

**In — this is the product.** A fast, private, local-first desktop mindmap:
capture, organize, persist, navigate, refine, export.

**Near — likely, in priority order.** Auto-layout, undo/redo, fold, search,
minimap, outline panel, theming, PNG/SVG/Markdown export, command palette,
autosave, presentation mode.

**Out — fenced, revisit only with a written reason.** Real-time collaboration,
cloud sync, accounts, plugins, mobile/web builds. Each of these either breaks
**P1** or freezes a data model that isn't ready.
