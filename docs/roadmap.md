# Roadmap

Where `mindmap-gui` is today and where it's going. This is the narrative
summary; the authoritative, intent-tracked task list is
[../TODOS.md](../TODOS.md), and the product design that justifies it all is
[../PYRAMID-OF-INTENTS.md](../PYRAMID-OF-INTENTS.md).

---

## Where we are

**v0.1 — working core editor.** You can launch it, create a tree of ideas,
arrange them freely on a zoomable canvas, and save your work as readable JSON.
The unsaved-changes guard protects you from losing anything to a stray close.

That is the whole product today. It is deliberately minimal, and the design
docs are deliberately not: the pyramid and todo list describe the full product
so every next step has a reason.

### Shipped capabilities

| Capability | Intent served |
| --- | --- |
| Create child / sibling node (`Tab`, `Enter`) | I1 Capture |
| Inline text editing, select-on-create (`F2`, double-click) | I1 Capture |
| Delete subtree (root protected) | I2 Organize |
| Free-form drag positioning | I2 Organize |
| Pan / zoom canvas, fit-to-map | I2 Organize, I4 Navigate |
| JSON save / open, versioned format | I3 Persist |
| Unsaved-changes guard on new/open/quit | I3 Persist |
| Open file via CLI argument | I3 Persist |

### Debt we're carrying

These are real limitations, not nits. Each is a task in Phase 1.

| Debt | Consequence | Task |
| --- | --- | --- |
| No undo/redo | a mis-click is unrecoverable | **T1.3** (highest value) |
| Node lookup is O(n) linear scan | will not scale to thousands of nodes | T1.1 |
| `Canvas::show` takes 6 args | fragile, hard to read | T1.2 |
| New-node placement is naive offsets | siblings can overlap | T1.4 |
| Node text measured every frame | frame cost grows with map size | T1.5 |
| Edge routing assumes horizontal bias | looks wrong for nodes above/left | T2.6 |
| Shortcuts hard-coded in an if/else | not rebindable, not palette-searchable | T4.1 |

---

## The plan, phase by phase

Each phase delivers a coherent increment. **Nothing from a later phase starts
before the earlier phase is done** — the early phases remove debt that later
features would multiply.

### Phase 1 — Core hardening (P0)

Make the foundation trustworthy. Close the debt above.

Undo/redo (T1.3) is the single highest-value item on the entire backlog — it
turns the app from "fragile" into "safe to experiment in". It lands alongside
the structural refactors (hash-indexed model, `EditorState` bundling) that make
it implementable, plus autosave/recovery (T1.6) and session restore (T1.7) so
**I3 Persist** is real.

*Exit criteria:* every mutation is undoable; a 1 000-node map performs; a crash
loses at most 5 s of work.

### Phase 2 — Organize & navigate (P1: I2, I4)

Fold/collapse, multi-select, clipboard, drag-to-reparent, arrow-key navigation,
search, minimap, outline panel, and auto-layout engines.

This is the phase that makes a *large* map usable rather than just a small one.
Auto-layout (T2.10) is the flagship: it replaces the naive placement heuristic
with real tree/radial/org-chart algorithms, opt-in per map or per branch.

*Exit criteria:* a 1 000-node map can be searched, folded, rearranged, and
re-laid-out without friction.

### Phase 3 — Refine & express (P1: I5, I6)

Per-node styling, theme colour inheritance, icons/markers, notes, hyperlinks,
light/dark themes — then the exporters: PNG, SVG, PDF, and Markdown/OPML/plain
outline, plus the matching importers.

Export/import is where the "honest file format" promise (P6) gets tested:
every importer is either lossless or reports exactly what it dropped.

*Exit criteria:* a map can leave the app in a form someone without the app can
read, and come back.

### Phase 4 — Platform & polish (P2: I7, P4, P5)

Command palette, rebindable keymap, context menus, presentation mode, an
accessibility pass, large-map soak tests in CI, packaging, and release
automation.

This is the phase that turns a capable tool into a *finished* one.

*Exit criteria:* `Ctrl+K` reaches everything; the app is installable on Linux,
macOS, and Windows from a tagged release.

### Phase 5 — Fenced (do not start without a written scope change)

| Fenced | Why |
| --- | --- |
| Real-time collaboration | needs a server + CRDT + accounts — violates **P1** (local-first) |
| Cloud sync | same |
| Plugin / scripting runtime | freezes the data model and the security story |
| Mobile / web build | desktop is the product |

These aren't "later" — they're "not without a written reason". Each one either
breaks the local-first promise or locks in a data model that isn't stable yet.

---

## Guiding constraints

Every phase is held to the pyramid's principles (**L2**):

| | Principle | What it rules out |
| --- | --- | --- |
| **P1** | Local-first & private | accounts, cloud, telemetry, required network |
| **P2** | Fluid — never block the canvas | synchronous I/O on the UI thread |
| **P3** | Forgiving — undo everything, autosave always | irreversible actions |
| **P4** | Keyboard-first, mouse-complete | mouse-only workflows |
| **P5** | Simple core, progressive disclosure | chrome that's always in the way |
| **P6** | Honest file format | binary/opaque/lossy storage |

And to the measurable budgets (**L4**): < 16 ms input latency, ≤ 5 s autosave,
< 50 ms auto-layout for 1 000 nodes, < 100 ms search, < 2 s PNG export.

---

## How to follow along

- **Want the big picture?** Read [../PYRAMID-OF-INTENTS.md](../PYRAMID-OF-INTENTS.md).
  Start at the apex (purpose) and work down; it's built to be read top-down.
- **Want to know what's next?** [../TODOS.md](../TODOS.md) — tasks are phased,
  sized (S/M/L), dependency-linked, and carry the `I#` intent IDs they serve.
- **Want to build something?** [development.md](development.md) has the
  toolchain, the check gauntlet, and a worked example (fold/collapse) showing
  how a task flows from intent to code to test.
- **Want to understand the code?** [architecture.md](architecture.md).

---

## Contributing

1. Pick a task from [../TODOS.md](../TODOS.md). Phase 1 first — it unblocks
   everything else.
2. Read the task's intent IDs and acceptance criteria. If the task looks
   different from what the criteria describe, say so before writing code.
3. Follow [development.md](development.md): place the code per the
   extension-points table, add tests for anything touching `model.rs`, and run
   the full check gauntlet (`fmt`, `clippy -D warnings`, `test`, `build --release`).
4. A task isn't done until its acceptance criteria are exercised by hand on a
   map with **more than 300 nodes** — not just the 1-node default.

Questions about *why* something is designed the way it is almost always have
the answer in the pyramid. If they don't, that's a gap in the design doc, and
fixing it is a contribution too.
