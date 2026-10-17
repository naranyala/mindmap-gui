# Development

Everything you need to build, test, and contribute to `mindmap-gui`.

---

## Toolchain

| Tool | Version | Notes |
| --- | --- | --- |
| Rust | 1.85+ (project uses **edition 2024**) | CI / reference builds use 1.98.1 |
| Cargo | ships with Rust | the only build system |
| OS | Linux, macOS, Windows | Linux needs X11 or Wayland; both feature flags are on by default |

No other system dependencies are required on a normal desktop. Linux file
dialogs use **XDG desktop portals** (GNOME/KDE/Sway all provide these). If you
want the GTK fallback, install `gtk3` dev headers — but you almost certainly
don't need to.

### Getting Rust

```bash
# recommended
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup update
```

Verify:

```bash
rustc --version   # expect 1.85 or newer
cargo --version
```

---

## Build & run

```bash
cargo build                     # debug build
cargo run                       # debug build + launch
cargo run --release             # optimized build + launch (use this to measure)
cargo run --release -- my.json  # launch and open a file
```

The first build compiles `egui`/`eframe`/`winit` and takes a few minutes.
Everything after that is incremental and fast.

> **Note:** there is currently no separate test binary or example. `cargo run`
> is the way to exercise the app.

---

## The check gauntlet

Nothing is "done" until all four pass. This is the definition of done from the
pyramid (L6) and the todo list.

```bash
cargo fmt --check                          # 1. formatting
cargo clippy --all-targets -- -D warnings  # 2. lints, warnings are errors
cargo test                                 # 3. unit tests
cargo build --release                      # 4. release build, zero warnings
```

One-liner to run the first three:

```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```

### Fixing formatting

```bash
cargo fmt          # rewrite files in place
cargo fmt --check  # verify without rewriting (what CI runs)
```

### Fixing lints

```bash
cargo clippy --all-targets                 # show warnings
cargo clippy --fix --all-targets           # auto-fix what's safe
```

`-D warnings` makes every Clippy warning a hard error, so the build fails
rather than scrolling past. That's intentional — the codebase is small enough
to hold to a clean bar.

---

## Tests

```bash
cargo test                        # run all
cargo test model::                # only the model tests
cargo test save_load_roundtrip    # one test
```

All tests live in `src/model.rs` under `#[cfg(test)] mod tests`. There are
currently **9 tests**, covering the tree operations and the file format:

| Test | What it proves |
| --- | --- |
| `default_map_has_root` | a fresh map has exactly one node, the root |
| `add_child_links_both_ways` | parent↔child pointers stay consistent |
| `add_sibling_inserts_after_and_root_sibling_is_child` | sibling ordering; root's sibling becomes a child |
| `delete_subtree_removes_descendants` | whole subtree removed, siblings untouched |
| `cannot_delete_root` | root protection works |
| `set_text_only_marks_changed_text` | `set_text` returns false when text is unchanged (used for dirty tracking) |
| `save_load_roundtrip` | serialize → deserialize → same tree |
| `restructure_drops_dangling_and_unreachable` | load-time repair prunes bad references |
| `restructure_rejects_missing_root` | a map with no root is refused |

### Testing strategy going forward

The model layer is the only layer with unit tests today, and that's deliberate:
it's pure data + tree rules, with no `egui` dependency, so it's cheap to test.
As features grow:

- **New tree operation** → add a unit test next to the existing ones. Cover
  the happy path *and* the boundary (e.g. deleting the root, empty children).
- **File format change** → add a round-trip test **and** a fixture file for the
  previous version that must still load.
- **Layout / placement** → property tests are the goal (task T1.4 requires a
  property test asserting no overlaps across 500 insertions).
- **UI / interaction** → currently verified by hand on a map with **> 300
  nodes**. Automated UI testing is not in scope yet; the manual bar exists so
  we don't fool ourselves with a 1-node smoke test.

---

## Project layout

```
mindmap-gui/
├── src/
│   ├── main.rs      eframe bootstrap, viewport, CLI argument
│   ├── app.rs       shell: menus, status bar, shortcuts, dialogs, file ops
│   ├── canvas.rs    Scene host: pan/zoom, node paint, hit-test, inline edit
│   └── model.rs     data model + JSON, with all unit tests
├── docs/
│   ├── architecture.md
│   ├── file-format.md
│   ├── keyboard-shortcuts.md
│   ├── development.md          ← you are here
│   └── roadmap.md
├── PYRAMID-OF-INTENTS.md       product design pyramid
├── TODOS.md                    task list with intent IDs
├── Cargo.toml
└── README.md
```

Read [architecture.md](architecture.md) before touching `src/`. The short version:

- `model.rs` is **pure** — no egui widgets, only `Pos2`/`Rect`/`Vec2` math types.
  Keep it that way; it's what makes the tests cheap.
- `app.rs` owns orchestration and all state.
- `canvas.rs` is the only file that paints and hit-tests.

---

## Adding a feature

1. **Trace the intent.** Open `PYRAMID-OF-INTENTS.md` and find the `I#` intent
   your feature serves. If you can't find one, it doesn't belong yet.
2. **Find the task.** `TODOS.md` has a phased list with `I#` IDs, sizes, and
   acceptance criteria. If your feature isn't listed, add it with its intent ID
   before building.
3. **Place the code** per the extension-points table in
   [architecture.md](architecture.md).
4. **Test it.** If it touches `model.rs`, add a unit test. If it's UI, be ready
   to demonstrate it on a 300+ node map.
5. **Run the gauntlet.** All four checks must pass.
6. **Update the docs.** If you changed behavior, shortcuts, or the file format,
   update the relevant file in `docs/`.

### A concrete example: adding fold/collapse

This is task **T2.1** in the roadmap, so it's a good template:

1. Intent: **I2** (Organize) and **I4** (Navigate).
2. `TODOS.md` T2.1: *"`folded` flag on node; hidden descendants not painted,
   not hit-tested, excluded from `bounds()`."*
3. Data: add `folded: bool` to `Node` with `#[serde(default)]` (a safe field
   addition — no version bump needed, see [file-format.md](file-format.md)).
   Keep it pure in `model.rs`.
4. Rendering: `canvas.rs` skips painting descendants of a folded node and
   excludes them from the measure pass.
5. Interaction: `app.rs` binds a key (planned `Space`/`Ctrl+.`) and routes
   through `model` so the change is a command (undoable, once T1.3 lands).
6. Test: a model test that folding hides the right IDs from a traversal.
7. Format: the round-trip test must still pass, and a v1 file without `folded`
   must load with `folded = false`.

---

## Debugging tips

**The canvas looks wrong / nodes are unclickable**

- Check the `Scene` layer transform. Everything inside `scene.show` is in
  *scene coordinates*; `ui.interact` and `painter` calls are automatically
  transformed. If you compute a rect by hand, make sure it's in scene space.
- `sizes` (node dimensions) are computed once per frame in the measure pass.
  If paint and interact disagree about a rect, the measure pass and the paint
  pass are out of sync.

**A shortcut doesn't fire**

- `shortcuts()` returns early if `pending.is_some() || editing.is_some()`. A
  stale `editing` value (e.g. from an uncommitted edit) will swallow every
  global chord.

**Save/load fails silently**

- Errors go to `status` in the status bar, not a panic. Check the bottom-left
  text.
- For a load failure, verify `map.root` points at a node that exists — the
  loader refuses a map with no root.

**The build feels slow**

- First build is the dependency tree. Use `cargo run --release` for anything
  you want to measure; debug builds of egui are noticeably slower.

---

## Dependencies

Declared in `Cargo.toml`:

| Crate | Version | Why |
| --- | --- | --- |
| `eframe` | 0.37 (glow backend, x11+wayland) | the app framework; provides the window, event loop, and `egui` |
| `egui` | 0.37 (with `serde`) | immediate-mode UI + the `Scene` canvas |
| `rfd` | 0.17 | native file dialogs |
| `serde` | 1.0 (with `derive`) | (de)serialization |
| `serde_json` | 0.15 | the JSON file format |

Nothing else. The pyramid's stack table (L6) lists what's planned: `resvg` +
`tiny-skia` for PNG/SVG export, a PDF crate, and `std::thread` for background
work — none of which are needed until those tasks start.

Adding a dependency? Justify it against principle **P1** (local-first, no
network) and **P2** (never block the canvas). A dependency that pulls in a
runtime or a network stack needs a strong reason.

---

## Releases

Not yet automated (task T4.9). Today a release is:

```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
cargo build --release
```

then ship the binary from `target/release/`. When T4.8/T4.9 land, `cargo-dist`
will produce platform artifacts + checksums from a git tag.
