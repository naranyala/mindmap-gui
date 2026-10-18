# Todos — Mindmap Desktop App

Actionable breakdown of the vision in
[`PYRAMID-OF-INTENTS.md`](./PYRAMID-OF-INTENTS.md). Every task carries the
**intent IDs (`I#`)** it serves, so nothing here exists without a reason.

## Legend

| Marker | Meaning |
|--------|---------|
| `[x]`  | shipped, in the code today |
| `[~]`  | in progress |
| `[ ]`  | todo |
| `[-]`  | fenced — blocked by a scoping decision, do not start |
| `S`/`M`/`L` | size: small (< ½ day) / medium (½–2 days) / large (3+ days) |

**Rule:** a task is only checked off when `TODOS.md`'s
[Definition of Done](#definition-of-done) holds.

---

## Phase 0 — MVP (shipped)

The current build. `cargo run` gives a usable single-map editor.

- [x] **T0.1** — Data model: `MindMap` / `Node` / `NodeId`, tree ops
      (`add_child`, `add_sibling`, `delete_subtree`, `restructure`, `bounds`).
      *I1 I2 I3* · `src/model.rs` · 9 unit tests green
- [x] **T0.2** — Canvas: `egui::Scene` pan/zoom (0.1–4×), cubic-bezier edges,
      node hit-testing, drag-to-move, select, inline `TextEdit` with
      select-on-create. *I1 I2* · `src/canvas.rs`
- [x] **T0.3** — Shell: menu bar, status bar, keyboard shortcuts,
      unsaved-changes `Modal` on new/open/quit, `CancelClose` interception.
      *I1 I3* · `src/app.rs`
- [x] **T0.4** — Persistence: JSON `FileDoc` v1, `rfd` open/save dialogs,
      open-via-CLI-arg, load-time `restructure` repair. *I3* · `src/model.rs`
- [x] **T0.5** — Design docs: pyramid of intents + this todo list.

### Debt carried out of Phase 0

Fix-forward, not optional — each is a real limitation, not a nitpick.

- [ ] **T0.D1** — `model::get`/`get_mut` are O(n) linear scans over
      `Vec<Node>`. *I2 I4* · Blocks large-map performance (**L6**). → T1.1
- [ ] **T0.D2** — `Canvas::show` takes **6 positional arguments**; the caller
      has to know the whole app state. Unreadable and fragile. → T1.2
- [ ] **T0.D3** — New-node placement is a naive `CHILD_DX`/`CHILD_GAP`
      offset; siblings can overlap when the parent already has many children
      or the user has dragged things around. *I1 I2* → T1.4
- [ ] **T0.D4** — Node galley/size measured for **every** node **every** frame.
      *I2* · Frame-cost scaling risk → T1.5
- [ ] **T0.D5** — Edge routing assumes horizontal child bias; edges look wrong
      for nodes placed above/left of their parent. *I2* → T2.6
- [ ] **T0.D6** — No undo/redo at all. *I1 I2 I3* → T1.3 (highest-value single
      item in the backlog)
- [ ] **T0.D7** — `shortcuts()` hard-codes every chord in an if/else chain;
      not user-rebindable, not searchable by a command palette. *P4* → T4.1

---

## Phase 1 — Core hardening (P0: make the foundation trustworthy)

Closes the debt above. Nothing else should start until Phase 1 is done —
every later feature multiplies these problems.

- [ ] **T1.1** — **Hash-indexed model.** `HashMap<NodeId, Node>` + stable
      `order: Vec<NodeId>`. Keep `Vec`-equivalent serialize order.
      *I2 I4* · `src/model/mod.rs` · **M** · Deps: T0.D1
      *Accept:* `get`/`get_mut` are O(1); existing 9 tests still pass; a
      5 000-node synthetic map loads and queries without quadratic slowdown.
- [ ] **T1.2** — **Introduce `EditorState`.** Bundle `map, selected, editing,
      dirty, canvas` into one struct; `Canvas::show(&mut self, ui, &mut EditorState)`.
      *P2* · `src/canvas.rs`, `src/app.rs` · **S** · Deps: T0.D2
      *Accept:* `Canvas::show` signature is ≤ 3 params; no behavior change.
- [ ] **T1.3** — **Undo / redo (command pattern).** `Command` enum with
      apply + invert, `History` stack (≥ 200 steps), 800 ms coalescing for
      `MoveNode`/`SetText`, `Batch` for multi-ops. Wire `Ctrl+Z` /
      `Ctrl+Shift+Z` + Edit menu.
      *I1 I2 I3* · `src/model/{command,history}.rs`, `src/app.rs` · **L** · Deps: T1.1, T1.2
      *Accept:* every mutation in the app is undoable; one continuous drag =
      one undo step; redo invalidated only by a genuinely new command;
      history survives inline-edit commit.
- [ ] **T1.4** — **Smart placement (overlap-free).** When adding a node,
      probe candidate offsets and pick the first that clears all existing
      nodes by a margin. Deterministic. *I1 I2* · `src/model/layout.rs` · **M** · Deps: T1.1
      *Accept:* adding 500 children to one parent produces zero overlaps;
      property test asserts pairwise rect separation.
- [ ] **T1.5** — **Layout caching + viewport culling.** Cache node
      size/galley keyed by `(text, font, zoom_bucket)`; invalidate on edit.
      Cull paint + hit-test to `scene_rect` inflated by a margin.
      *P2 I2* · `src/canvas.rs` · **M** · Deps: T1.2
      *Accept:* steady-state frame on a 1 000-node map does no text layout;
      off-screen nodes are not painted (verified via debug counter).
- [ ] **T1.6** — **Autosave + crash recovery.** Shadow file beside the
      document (or in a per-user state dir for untitled maps), written ≤ 5 s
      after the last edit on a worker thread. On launch, if a newer shadow
      exists, offer recovery. *I3* · `src/io/autosave.rs` · **M** · Deps: T1.3
      *Accept:* `kill -9` during editing loses ≤ 5 s of work; recovery prompt
      is dismissible without corrupting either copy.
- [ ] **T1.7** — **Session restore + recent files.** Persist window size,
      last map path, camera, selection, and a recent-files list. Full
      "exactly as left" also needs fold state — lands once `folded`
      exists and round-trips via format v2's `view` (T1.8) and T2.1.
      *I3* · `src/io/session.rs` · **S** · Deps: T1.1, T1.8
      *Accept:* quit and relaunch → same map, same zoom/pan/selection;
      after T2.1, fold state restores too.
- [ ] **T1.8** — **File format v2.** Add `note`, `style`, `folded`, `link`,
      `icon`, `view`, `meta` with serde defaults; keep v1 files loading;
      round-trip unknown fields byte-stably. *I3 I5* · `src/model/format.rs` · **M** · Deps: T1.1
      *Accept:* v1 fixture loads; v2 with unknown extra field re-saves with
      the field intact; explicit version gate with a clear error message.
- [ ] **T1.9** — **Off-thread document save.** `Ctrl+S`/Save As serialize on
      a worker thread against a snapshot (same machinery as T1.6's shadow
      write), temp file + atomic rename; the UI keeps editing while the
      write completes and a newer save supersedes it.
      *I3 P2* · `src/io/save.rs` · **M** · Deps: T1.6
      *Accept:* saving a 5 000-node map never blocks input < 16 ms; edits
      made during a save are captured by the following save; no torn file
      if the process dies mid-write (temp + rename).

---

## Phase 2 — Organize & navigate (P1: I2, I4)

- [ ] **T2.1** — **Fold / expand subtree.** `folded` flag on node; hidden
      descendants not painted, not hit-tested, excluded from `bounds()`.
      Fold indicator on nodes with children; folding is a `SetFolded`
      command so it joins the undo history. *I2 I4* · **M** · Deps: T1.1, T1.8, T1.3
- [ ] **T2.2** — **Multi-select.** Shift-click additive, marquee on canvas
      drag, `Ctrl+A` select all. Selection is a `HashSet<NodeId>`.
      *I2* · `src/canvas/interaction.rs` · **L** · Deps: T1.2, T1.3
- [ ] **T2.3** — **Clipboard.** Copy/cut/paste/duplicate selection; paste
      targets the selected node or the root; internal + system clipboard via
      serialized subtree JSON. *I1 I2* · **M** · Deps: T2.2, T1.3
- [ ] **T2.4** — **Drag to re-parent.** Dragging node A over node B makes B
      A's parent (cycle-guarded); dropping on empty canvas detaches to root.
      Re-parent is a `Reparent` command (undo restores parent + index).
      *I2* · `src/canvas/interaction.rs` · **M** · Deps: T2.2, T1.3
      *Accept:* cannot create a cycle (ancestor dropped on descendant) — show
      a refused-drop cursor instead.
- [ ] **T2.5** — **Arrow-key navigation.** Nearest node in the pressed
      direction from the current selection; `Shift`+arrow extends selection.
      *I2 I4* · **M** · Deps: T2.2
- [ ] **T2.6** — **Direction-aware edge routing.** Anchor and control points
      chosen from relative position (child left/right/above/below), not a
      fixed horizontal bias. *I2* · `src/canvas/edge.rs` · **M** · Deps: T0.D5
- [ ] **T2.7** — **Search / find.** `Ctrl+F` incremental substring + regex;
      highlight matches; `Enter` jumps camera to next hit, wrapping.
      *I4* · `src/model/query.rs`, `src/ui/palette.rs` · **M** · Deps: T1.1
      *Accept:* 1 000-node search updates < 100 ms per keystroke.
- [ ] **T2.8** — **Minimap.** Corner thumbnail with viewport rect; click/drag
      to pan; LOD'd and culled so it costs < 2 ms. *I4* · **M** · Deps: T1.5
- [ ] **T2.9** — **Outline / list panel.** Two-way-synced indented tree side
      panel; selects/folds/edits in sync with the canvas.
      *I4* · `src/ui/outline.rs` · **L** · Deps: T2.1, T1.3
- [ ] **T2.10** — **Auto-layout engines.** Tree (LR/RL/TB), classic mindmap,
      radial, org-chart. Opt-in per map or per branch. Chunked for large maps.
      *I2* · `src/model/layout.rs` · **L** · Deps: T1.1, T1.3
      *Accept:* 1 000 nodes layout in < 50 ms off the frame path; result is
      undoable as one `Batch`.
- [ ] **T2.11** — **Detail LOD (zoom-aware painting).** Below a zoom
      threshold draw nodes as filled rects and edges as straight segments —
      no text shaping, no galley reads (L6 strategy table). The threshold
      reuses T1.5's `zoom_bucket`.
      *P2 I2 I4* · `src/canvas/node_widget.rs` · **M** · Deps: T1.5
      *Accept:* a 1 000-node map at 0.15× does zero `layout_no_wrap` calls
      in steady state (debug counter) and frame cost stays flat.

---

## Phase 3 — Refine & express (P1: I5, I6)

- [ ] **T3.1** — **Node styling.** Per-node colour, border, corner radius;
      inspector panel to edit. Style edits are `SetStyle` commands so a
      change is undoable in one step (L4 I5 budget). *I5* · **M** · Deps: T1.8, T1.3
- [ ] **T3.2** — **Theme colour inheritance.** Branches inherit an accent
      from an ancestor unless overridden. *I5* · **S** · Deps: T3.1
- [ ] **T3.3** — **Icons & markers.** Priority (1–5), progress, flag, custom
      emoji; rendered in the node box; icon edits are commands like all
      mutations. *I5* · **M** · Deps: T1.8, T1.3
- [ ] **T3.4** — **Node notes.** Markdown `note` field, shown in inspector,
      rendered in a popover on hover/click; note edits are `SetNote`
      commands. *I5* · **M** · Deps: T1.8, T1.3
- [ ] **T3.5** — **Hyperlinks.** `link` field, open in system browser.
      *I5 I8* · **S** · Deps: T1.8
- [ ] **T3.6** — **Themes.** Light / dark / system + accent picker; egui
      `Visuals` wired from `model::style`. *I5* · **M** · Deps: T3.1
- [ ] **T3.7** — **Export PNG / SVG.** Render the full map bounds at 1×/2×/4×
      via `resvg` + `tiny-skia`. *I6* · **L** · Deps: T2.6
      *Accept:* 1 000-node export < 2 s; exported image matches on-screen
      layout (side-by-side diff on a fixture map).
- [ ] **T3.8** — **Export PDF.** Vector, print-ready, multi-page if needed.
      *I6* · **M** · Deps: T3.7
- [ ] **T3.9** — **Export outline text.** Markdown / OPML / plain text,
      respecting fold state (export folded branches collapsed or expanded,
      user choice). *I6 I8* · **M** · Deps: T2.1
- [ ] **T3.10** — **Import Markdown / OPML / Freeplane `.mm`.** Heading depth
      → tree depth. Every importer either round-trips or reports exactly what
      it dropped. *I8* · **L** · Deps: T1.8
      *Accept:* round-trip test in CI for each format — export then re-import
      yields an identical tree (text + structure).

---

## Phase 4 — Platform & polish (P2: I7, P4, P5)

- [ ] **T4.1** — **Keymap table.** Named actions → chords in one table;
      `input/keymap.rs`; powers the palette, the help window, and rebinding.
      *P4* · **M** · Deps: T0.D7
- [ ] **T4.2** — **Command palette.** `Ctrl+K` fuzzy search over every named
      action + node search. *P4 P5* · **M** · Deps: T4.1, T2.7
- [ ] **T4.3** — **Context menus.** Right-click on node and on canvas.
      *P4 P5* · **S** · Deps: T4.1
- [ ] **T4.4** — **Customizable shortcuts.** Rebind via a settings view;
      persisted in session state. *P4* · **M** · Deps: T4.1
- [ ] **T4.5** — **Presentation mode.** Fullscreen, no editing chrome,
      precomputed camera targets + animated transitions (no reflow while
      advancing, L4 I7), step through in reading order.
      *I7* · **L** · Deps: T2.10
- [ ] **T4.6** — **Accessibility pass.** Focus order, screen-reader labels on
      nodes (egui AccessKit), high-contrast theme, visible focus rings.
      *P4* · **M** · Deps: T3.6
- [ ] **T4.7** — **Large-map soak test.** Generated 5 000-node fixture in CI;
      assert the **L4** budgets explicitly: cold start < 500 ms (I1),
      worst-case input latency < 16 ms (P2), search < 100 ms (I4), export
      < 2 s (I6), plus steady-state frame budget.
      *P2 I1 I4 I6* · **M** · Deps: T1.5
- [ ] **T4.8** — **Packaging.** `.desktop` entry + icon on Linux, `.app`/
      `.dmg` on macOS, `.msi`/`.exe` on Windows; register `.mindmap` file
      association. *I3* · **M** · Deps: T1.8
- [ ] **T4.9** — **`cargo-dist` / release automation.** Tagged releases
      produce platform artifacts + checksums. *I3* · **S** · Deps: T4.8
- [ ] **T4.10** — **Local-first & privacy audit (P1 guardrail).** Prove the
      release binary needs no network: no network crate reachable from the
      default feature set, a CI source scan that fails if one is added
      under `src/`, and a manual run with networking disabled
      (`unshare -n cargo run`) against a > 300-node map.
      *P1* · **S** · No deps
      *Accept:* open/edit/save works with all networking disabled; the CI
      guard is green.
- [ ] **T4.11** — **Help window from the keymap.** Searchable list of every
      named action + its chord (or "unassigned"), generated from
      `input/keymap.rs` so it can never drift from real behavior.
      *P4* · `src/ui/help.rs` · **S** · Deps: T4.1
      *Accept:* every keyboard-reachable action appears in the window;
      rebinding a chord (T4.4) updates it automatically.

---

## Phase 5 — Fenced (do not start without a written scope change)

- [ ] **T5.1** — `[-]` Real-time collaboration *(I9)* — violates **P1**;
      requires server + CRDT + accounts.
- [ ] **T5.2** — `[-]` Cloud sync *(I9)* — violates **P1**.
- [ ] **T5.3** — `[-]` Plugin / scripting runtime *(I10)* — freezes the data
      model and the security story.
- [ ] **T5.4** — `[-]` Mobile / web build — desktop is the product.

---

## Definition of done

A task is complete only when **all** of these hold:

1. `cargo build --release` — clean, zero warnings.
2. `cargo clippy --all-targets -- -D warnings` — clean.
3. `cargo test` — green, including any new tests the task requires.
4. The task's *Accept* criteria are exercised by hand on a map with **> 300
   nodes** (not just the 1-node default).
5. The task's intent IDs above are updated in
   [`PYRAMID-OF-INTENTS.md`](./PYRAMID-OF-INTENTS.md) → **L5** status column.
6. `cargo fmt` applied.

## Verification commands

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
cargo run -- <some-map.json>     # manual pass, > 300 nodes
```

## Quick reference — intent → tasks

| Intent | Tasks |
|--------|-------|
| **I1** Capture | T0.1–T0.3 · T1.3 T1.4 · T2.3 · T4.7 |
| **I2** Organize | T0.1–T0.2 · T1.1–T1.5 · T2.1–T2.6 T2.9–T2.11 |
| **I3** Persist | T0.4 · T1.3 T1.6–T1.9 · T4.8 T4.9 |
| **I4** Navigate | T1.1 T1.5 · T2.7–T2.9 T2.11 · T4.7 |
| **I5** Refine | T1.8 · T3.1–T3.6 |
| **I6** Express | T2.6 · T3.7–T3.9 |
| **I7** Present | T4.5 |
| **I8** Extend | T3.5 T3.9 T3.10 |
| **I9/I10** Fenced | T5.1–T5.4 |
