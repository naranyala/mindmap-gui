# Mindmap

A fast, private, local-first **desktop mind-mapping tool** written in Rust with
[egui/eframe](https://github.com/emilk/egui). Create a thought, shape it, and
keep it on your disk as a plain JSON file — no account, no cloud, no telemetry.

> Status: **v0.1 — working core editor.** Create/edit/delete nodes, free-form
> canvas with pan & zoom, JSON save/open with a versioned format, unsaved-changes
> guard. See the [roadmap](docs/roadmap.md) for where it's going.

---

## Features

**Shipped today**

- **Canvas** — `egui::Scene`-based infinite canvas with smooth pan and zoom
  (0.1×–4×), cubic-bezier edges, and crisp hit-testing.
- **Nodes** — add children/siblings, inline text editing with select-on-create,
  drag to reposition, subtree delete (root is protected).
- **Keyboard-first** — Tab to add a child, Enter to add a sibling, F2 to edit,
  and everything else from a single hand. Full list in
  [docs/keyboard-shortcuts.md](docs/keyboard-shortcuts.md).
- **Persistence** — versioned JSON format (`FileDoc` v1) with load-time repair,
  native file dialogs, open-from-CLI, and an unsaved-changes guard on
  new/open/quit.
- **Theming** — follows your system light/dark preference automatically.

**Planned** — undo/redo, fold/collapse, multi-select, auto-layout, search,
minimap, outline panel, command palette, autosave/recovery, PNG/SVG/PDF/Markdown
export. Tracked in [TODOS.md](TODOS.md) and prioritized by
[PYRAMID-OF-INTENTS.md](PYRAMID-OF-INTENTS.md).

---

## Install & run

### Prerequisites

- **Rust** 1.85+ (the project uses edition 2024; CI builds with 1.98.1).
- On **Linux**, a working graphics stack. The build enables both `x11` and
  `wayland`. Native file dialogs (`rfd`) use XDG desktop portals, which is what
  GNOME/KDE/Sway already provide — no extra packages needed on a normal desktop.
  If you build from a bare environment and want the GTK fallback instead, install
  `gtk3` development headers.

### Build & run

```bash
git clone <repo-url> && cd mindmap-gui

cargo run --release          # build and launch
cargo run --release -- my-map.json   # launch and open a file
```

The first build compiles the `egui`/`eframe`/`winit` stack and takes a few
minutes; subsequent builds are fast.

### Just check it compiles

```bash
cargo build
```

---

## Usage

Launch and you get a single **"Central Topic"** node on an empty canvas. That's
the whole first screen — by design.

| Do this | Result |
| --- | --- |
| `Tab` | add a child under the selected node (starts editing immediately) |
| `Enter` | add a sibling after the selected node |
| `F2` / double-click | edit the selected node's label (`Enter` commits, `Esc` reverts) |
| `Delete` | delete the selected node **and all of its descendants** |
| drag a node | move it anywhere on the canvas |
| drag the background | pan |
| scroll | pan vertically/horizontally |
| `Ctrl` + scroll | zoom around the cursor |
| `Ctrl+S` / `Ctrl+O` / `Ctrl+N` | save / open / new (with save-guard) |

Full shortcut reference: **[docs/keyboard-shortcuts.md](docs/keyboard-shortcuts.md)**.

Your work is saved as **JSON** — open the file in a text editor any time.
Format details: **[docs/file-format.md](docs/file-format.md)**.

---

## Project layout

```
mindmap-gui/
├── src/
│   ├── main.rs      eframe bootstrap, viewport setup, CLI argument
│   ├── app.rs       shell: menus, status bar, shortcuts, dialogs, file ops
│   ├── canvas.rs    Scene host: pan/zoom, node paint, hit-test, inline edit
│   └── model.rs     data model: MindMap/Node, tree ops, JSON (de)serialization
├── docs/            architecture, file format, shortcuts, development, roadmap
├── PYRAMID-OF-INTENTS.md   product design: purpose → principles → intents → features
└── TODOS.md                actionable task list, phased and intent-traced
```

Design rationale lives in **[PYRAMID-OF-INTENTS.md](PYRAMID-OF-INTENTS.md)**;
the build plan lives in **[TODOS.md](TODOS.md)**; module internals are explained
in **[docs/architecture.md](docs/architecture.md)**.

---

## Documentation

| Document | What's in it |
| --- | --- |
| [docs/architecture.md](docs/architecture.md) | modules, data flow, the `egui::Scene` rendering model, threading |
| [docs/file-format.md](docs/file-format.md) | JSON schema, versioning & migration, load-time repair rules |
| [docs/keyboard-shortcuts.md](docs/keyboard-shortcuts.md) | complete keymap table |
| [docs/development.md](docs/development.md) | toolchain, build/test/clippy workflow, how to add a feature, testing strategy |
| [docs/roadmap.md](docs/roadmap.md) | phase-by-phase plan, current status, how to contribute |
| [PYRAMID-OF-INTENTS.md](PYRAMID-OF-INTENTS.md) | full product design pyramid |
| [TODOS.md](TODOS.md) | tracked tasks with intent IDs and acceptance criteria |

---

## Guarantees

These are the promises the design is held to (from the pyramid's **L4**):

- **Local-first & private.** Files are yours, on your disk, in readable JSON.
  No network access for any core feature.
- **Never lose work.** Unsaved-changes guard on every destructive action;
  autosave and crash recovery are next on the roadmap.
- **Fluid.** No synchronous I/O on the UI thread; input latency budget under
  16 ms.
- **Undoable.** Every mutation is designed to be undoable — undo/redo is the
  highest-priority item on the roadmap.

---

## Contributing

Read [docs/development.md](docs/development.md) first (toolchain, workflow,
testing bar), then pick a task from [TODOS.md](TODOS.md). Tasks carry intent
IDs so you can always trace *why* something is being built. A change isn't done
until `cargo fmt`, `cargo clippy -- -D warnings`, and `cargo test` are all clean.

---

## License

TBD — add a `LICENSE` file before publishing.
