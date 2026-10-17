# Keyboard Shortcuts

The app is keyboard-first (principle **P4**). Every core action is reachable
without the mouse, using one hand where possible.

Shortcuts are handled in two places and are **context-sensitive**:

- `src/app.rs::shortcuts()` — global chords (files, zoom, structure). Skipped
  entirely while a text edit or a modal dialog is active.
- `src/canvas.rs` — while editing a node's label (`Enter`/`Escape` commit or
  cancel the edit).

When a **modal** (unsaved-changes dialog) is open, all global shortcuts are
disabled until you dismiss it. When you are **editing a node label**, only the
editing-specific keys below apply — global chords are suppressed so typing
`Tab` or `Delete` edits your text instead of mutating the map.

---

## Editing the map

| Key | Action | Notes |
| --- | --- | --- |
| `Tab` | Add a **child** of the selected node | New node is created and editing starts immediately with all text selected — just type. |
| `Enter` | Add a **sibling** after the selected node | Same: created + editing starts. |
| `F2` | Edit the selected node's label | Equivalent to double-click. |
| `Delete` | Delete selected node **and its descendants** | Root is protected — see below. |
| `Backspace` | Same as `Delete` | Provided for keyboards without a dedicated Delete key. |
| `Escape` | Deselect the current node | Also cancels an in-flight edit (see below). |

### While editing a node label

| Key | Action |
| --- | --- |
| `Enter` | Commit the text and stop editing |
| `Escape` | Revert to the previous text and stop editing |
| *(click elsewhere)* | Commit |
| *(focus lost)* | Commit |

A new node starts editing with its placeholder text fully selected, so the
first keystroke replaces it entirely.

### Selecting

- **Click** a node to select it.
- **Double-click** a node to select and immediately edit it.
- **Click the empty canvas** to deselect (`Escape` does the same).

There is no multi-select or shift-click range selection yet — tracked as
task T2.2 in the roadmap.

---

## Files

| Key | Action |
| --- | --- |
| `Ctrl` + `N` | New map (prompts to save first if there are unsaved changes) |
| `Ctrl` + `O` | Open… (native file dialog) |
| `Ctrl` + `S` | Save — or **Save As…** if the map has never been saved |
| `Ctrl` + `Q` | Quit (prompts to save first if dirty) |

`Ctrl` is the command modifier on Linux and Windows; on macOS it is the `⌘`
key (egui maps `modifiers.command` to the platform convention automatically).

On quit, the OS close request is intercepted when dirty: the window stays open
and the unsaved-changes dialog appears instead. Nothing is ever discarded
without an explicit choice.

---

## View & canvas

| Key | Action |
| --- | --- |
| `Ctrl` + `=` (or `+`) | Zoom in (×1.25) |
| `Ctrl` + `-` | Zoom out (÷1.25) |
| `Ctrl` + `0` | Fit all nodes in view |

Zoom is clamped to **10 %–400 %** of the natural size.

### Pointer

| Gesture | Action |
| --- | --- |
| Drag a node | Move it |
| Drag the background | Pan |
| Middle-drag anywhere | Pan |
| Scroll | Pan (vertical / horizontal) |
| `Ctrl` + scroll | Zoom around the cursor |

---

## Menus

Everything above is also reachable from the menu bar:

- **File** — New, Open…, Save, Save as…, Quit (with shortcut hints shown)
- **View** — Zoom in, Zoom out, Fit all nodes
- **Help** — Keyboard shortcuts (opens this table in-app)

---

## Protected actions

Two rules prevent accidental damage:

1. **The root node cannot be deleted.** `Delete` on the root shows
   *"The central node can't be deleted"* in the status bar instead of deleting.
2. **Destructive file actions are guarded.** New, Open, and Quit show an
   *Unsaved changes* dialog when the map is dirty, offering **Save** /
   **Don't save** / **Cancel**.

---

## What's not here yet

The roadmap adds these keyboard-driven features:

| Feature | Keys (planned) | Task |
| --- | --- | --- |
| Undo / redo | `Ctrl+Z` / `Ctrl+Shift+Z` | T1.3 |
| Fold / expand subtree | `Space` or `Ctrl+.` | T2.1 |
| Multi-select | `Shift`+click, `Ctrl+A` | T2.2 |
| Copy / cut / paste / duplicate | `Ctrl+C/X/V/D` | T2.3 |
| Search / find | `Ctrl+F` | T2.7 |
| Command palette | `Ctrl+K` | T4.2 |
| Arrow-key navigation between nodes | `↑ ↓ ← →` | T2.5 |
| Rebindable shortcuts | via settings | T4.4 |

See [../TODOS.md](../TODOS.md) for the full plan and
[../PYRAMID-OF-INTENTS.md](../PYRAMID-OF-INTENTS.md) for why each matters.
