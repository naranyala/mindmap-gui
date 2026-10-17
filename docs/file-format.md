# File Format

Mind maps are saved as **human-readable JSON**. Open any map in a text editor
and you can read and hand-edit it. This is a deliberate product promise
(principle **P6** in the pyramid: *honest file format*).

Current on-disk format: **version 1**. Defined in `src/model.rs` as `FileDoc`.

---

## Top-level document

```json
{
  "app": "mindmap-gui",
  "version": 1,
  "map": {
    "root": 1,
    "next_id": 4,
    "nodes": [
      { "id": 1, "parent": null, "children": [2, 3], "pos": { "x": 0.0, "y": 0.0 }, "text": "Central Topic" },
      { "id": 2, "parent": 1, "children": [], "pos": { "x": 280.0, "y": 0.0 }, "text": "Research" },
      { "id": 3, "parent": 1, "children": [], "pos": { "x": 280.0, "y": 76.0 }, "text": "Write-up" }
    ]
  }
}
```

| Field | Type | Meaning |
| --- | --- | --- |
| `app` | `string` | Always `"mindmap-gui"`. Identifies the producer; ignored on load. |
| `version` | `u32` | Format version. Loader rejects versions **greater** than the app supports. |
| `map` | `MindMap` | The document body. |

---

## `MindMap`

| Field | Type | Meaning |
| --- | --- | --- |
| `root` | `NodeId` | ID of the central node. Must exist or the load fails. |
| `nodes` | `Node[]` | Every node in the map. Order is preserved on save but is **not** semantically meaningful. |
| `next_id` | `u64` | Monotonic counter; the next ID handed out by `insert_node`. |

`next_id` is stored so that IDs are never reused even after deletions — this
keeps external references (and future undo history) stable. It is
recomputed-safely: if a hand-edited file sets it too low, new nodes could
collide, so a future version may validate it against `max(id) + 1`.

---

## `Node`

| Field | Type | Meaning |
| --- | --- | --- |
| `id` | `NodeId` (`u64`) | Unique, never reused. |
| `parent` | `NodeId \| null` | `null` for the root. |
| `children` | `NodeId[]` | Ordered children. **Source of truth** for tree structure (see [Repair](#repair-on-load)). |
| `pos` | `{ "x": f32, "y": f32 }` | Center position in scene coordinates (points, not pixels — independent of zoom). |
| `text` | `string` | The node's label. May be empty. |

### What is *not* in v1

Version 1 stores only structure, position, and label. The roadmap adds these
as **backward-compatible optional fields** (task T1.8, format v2), each with a
serde default so v1 files continue to load unchanged:

| Planned field | Type | Purpose |
| --- | --- | --- |
| `note` | `string` | Markdown detail body (task T3.4) |
| `style` | `NodeStyle` | Per-node colour/border/shape (task T3.1) |
| `folded` | `bool` | Collapsed subtree state (task T2.1) |
| `link` | `string \| null` | Hyperlink (task T3.5) |
| `icon` | `Icon \| null` | Marker/priority icon (task T3.3) |
| `view` | `ViewportState` | Camera + selection, for session restore (task T1.8) |
| `meta` | `MapMeta` | Title, created/modified timestamps |

---

## Coordinates

`pos` is the node's **center** in scene space. Scene space is unbounded and
zoom-independent; the canvas maps it to screen coordinates via the `Scene`
transform each frame. Storing scene coordinates (not screen) means a map looks
the same regardless of the window size or the zoom level it was saved at.

Default layout on a fresh map: the root sits at `(0, 0)`. A new child of a
parent at `P` is placed at `P + (280, n × 76)` where `n` is how many children
the parent already has.

---

## Versioning & migration

The loader enforces a **forward-only** rule:

```rust
if doc.version > FILE_VERSION {
    return Err(format!(
        "file version {} is newer than supported version {}",
        doc.version, FILE_VERSION
    ));
}
```

- **Older version** → migrate forward on load. (No migrations exist yet; v1 is
  the first format. When v2 lands, `load_from_str` gains a `match doc.version`
  that upgrades v1 → v2 in memory.)
- **Same version** → load as-is.
- **Newer version** → refuse, with a message naming both versions. Never
  silently drop fields the user's newer app wrote.

### Adding a field (the safe recipe)

1. Add the field to the `struct` with `#[serde(default)]`.
2. Do **not** bump `FILE_VERSION` — a pure addition still loads on old readers
   (they ignore the unknown field) and old files still load on new readers
   (the default fills in).
3. Bump `FILE_VERSION` only when a change is **breaking**: a field is renamed,
   its type changes, or its semantics change such that old code would
   misinterpret it.
4. Add a round-trip test: serialize → deserialize → assert equality, plus a
   fixture file for the old version that must still load.

---

## Repair on load

Files can be hand-edited, truncated, or produced by a buggy third party. The
loader never trusts the input. `MindMap::restructure` runs on every load and
applies these rules **in order**:

1. **Validate root.** If no node has `id == map.root`, the load fails. There is
   no sensible map without a root.
2. **Prune dangling children.** Any ID in a node's `children` that does not
   correspond to an existing node is dropped. Self-references are dropped too.
3. **Reachability sweep.** BFS from `root` following `children`; any node not
   reached is deleted (they are orphans and invisible to the user anyway).
4. **Rebuild parents.** Every node's `parent` is recomputed from whoever lists
   it as a child. The `children` lists are authoritative; `parent` is derived.

The net effect: whatever you hand-edit in `children`, the app will produce a
consistent tree from it, and anything unreachable disappears rather than
silently corrupting the map.

`restructure` returns `bool` so the caller can distinguish "loaded fine" from
"had a missing root".

---

## Errors

All load/save errors surface as a single `String` shown in the status bar:

| Situation | Message |
| --- | --- |
| Malformed JSON | the `serde_json` parse error |
| Version too new | `file version N is newer than supported version M` |
| No root node | `file has no valid root node` |
| I/O failure | the `std::io::Error` text |

There is no error-recovery UI yet (e.g. "open a backup?"). Autosave and crash
recovery (task T1.6) will add a shadow file, so a future version can offer to
load the last good autosave when the primary file fails to parse.

---

## Interoperability

The format is intentionally plain so other tools can read it:

- **Language-agnostic** — any JSON parser reads it; no Rust-specific encoding
  beyond the `NodeId` newtype, which serializes as a bare `u64`.
- **Stable ordering** — `serde_json::to_string_pretty` with default key order;
  keys appear in struct-declaration order, so diffs between saves are readable.
- **Planned bridges** — Markdown, OPML, and Freeplane `.mm` import/export are
  tracked as tasks T3.9 and T3.10; they will be lossless or will report exactly
  what they drop.

---

## Full schema (v1)

```
FileDoc      := { app: string, version: u32, map: MindMap }
MindMap      := { root: NodeId, nodes: Node[], next_id: u64 }
Node         := { id: NodeId, parent: NodeId|null, children: NodeId[],
                  pos: {x: f32, y: f32}, text: string }
NodeId       := u64
```
