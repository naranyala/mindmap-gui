//! Undo/redo stacks (T1.3): each entry pairs the inverse command with the
//! user-facing action label chosen when the forward command ran.

use std::time::{Duration, Instant};

use super::command::Command;
use super::{MindMap, NodeId};

/// Retained undo steps (L4 I2: ≥ 200 steps).
const LIMIT: usize = 200;

/// Consecutive move/rename commands on the same node within this window merge
/// into a single undo step (so one continuous drag = one step).
const COALESCE_WINDOW: Duration = Duration::from_millis(800);

#[derive(Clone, Copy, PartialEq, Eq)]
enum MergeKind {
    Move,
    Rename,
    Details,
}

#[derive(Default)]
pub struct History {
    undo: Vec<(Command, &'static str)>,
    redo: Vec<(Command, &'static str)>,
    /// The step a follow-up command may coalesce into, if the top of the undo
    /// stack is still that step.
    last: Option<(NodeId, MergeKind, Instant)>,
}

impl History {
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn run(&mut self, cmd: Command, map: &mut MindMap) -> bool {
        self.run_at(cmd, map, Instant::now())
    }

    /// Applies `cmd` and records its inverse. Returns true if the map changed.
    pub fn run_at(&mut self, cmd: Command, map: &mut MindMap, now: Instant) -> bool {
        if let Some(mut changed) = self.try_merge(&cmd, now) {
            // The recorded step already covers the inverse; still apply the
            // actual map mutation (a merge is not a substitute for it).
            if cmd.run(map).is_some() {
                changed = true;
            }
            return changed;
        }
        let Some(inverse) = cmd.run(map) else {
            return false;
        };
        let label = cmd.describe();
        self.undo.push((inverse, label));
        if self.undo.len() > LIMIT {
            self.undo.remove(0);
        }
        // Only a genuinely new command invalidates redo; a coalesced extension
        // never reaches this path.
        self.redo.clear();
        self.last = merge_key(&cmd, now);
        true
    }

    /// Undoes the newest step. Returns its action label, if any.
    pub fn undo(&mut self, map: &mut MindMap) -> Option<&'static str> {
        let (cmd, label) = self.undo.pop()?;
        self.last = None;
        let forward = cmd.run(map)?;
        self.redo.push((forward, label));
        Some(label)
    }

    /// Redoes the most recently undone step. Returns its action label, if any.
    pub fn redo(&mut self, map: &mut MindMap) -> Option<&'static str> {
        let (cmd, label) = self.redo.pop()?;
        self.last = None;
        let inverse = cmd.run(map)?;
        self.undo.push((inverse, label));
        if self.undo.len() > LIMIT {
            self.undo.remove(0);
        }
        Some(label)
    }

    /// Returns `Some(changed)` when `cmd` extended the top step (e.g. a drag
    /// continuing the previous frame's move), `None` when it can't merge and
    /// must be pushed as a new step.
    fn try_merge(&mut self, cmd: &Command, now: Instant) -> Option<bool> {
        let (id, kind, at) = self.last?;
        match (kind, cmd) {
            (MergeKind::Move, Command::MoveNode { id: cmd_id, .. })
            | (MergeKind::Rename, Command::SetText { id: cmd_id, .. })
            | (MergeKind::Details, Command::SetDetails { id: cmd_id, .. })
                if *cmd_id == id => {}
            _ => return None,
        }
        if now.duration_since(at) > COALESCE_WINDOW {
            return None;
        }
        let (top, _) = self.undo.last_mut()?;
        let changed = match (kind, top, cmd) {
            // `top` is the stored inverse: its `old_pos`/`old` field mirrors
            // the forward action's final state and is what the extension
            // updates; its revert target must stay at the original position.
            (
                MergeKind::Move,
                Command::MoveNode {
                    old_pos: top_old, ..
                },
                Command::MoveNode { new_pos, .. },
            ) => {
                if top_old == new_pos {
                    false
                } else {
                    *top_old = *new_pos;
                    true
                }
            }
            (
                MergeKind::Rename,
                Command::SetText { old: top_old, .. },
                Command::SetText { new, .. },
            ) => {
                if top_old == new {
                    false
                } else {
                    *top_old = new.clone();
                    true
                }
            }
            (
                MergeKind::Details,
                Command::SetDetails { old: top_old, .. },
                Command::SetDetails { new, .. },
            ) => {
                if top_old == new {
                    false
                } else {
                    *top_old = new.clone();
                    true
                }
            }
            _ => false,
        };
        // Even a no-op extension refreshes the coalescing window.
        self.last = Some((id, kind, now));
        Some(changed)
    }
}

/// What a follow-up command would need to match to coalesce with `cmd`.
fn merge_key(cmd: &Command, now: Instant) -> Option<(NodeId, MergeKind, Instant)> {
    match cmd {
        Command::MoveNode { id, .. } => Some((*id, MergeKind::Move, now)),
        Command::SetText { id, .. } => Some((*id, MergeKind::Rename, now)),
        Command::SetDetails { id, .. } => Some((*id, MergeKind::Details, now)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Pos2, Vec2};

    fn base() -> (MindMap, NodeId) {
        let mut map = MindMap::default();
        let id = map.add_child(map.root).expect("child under root");
        (map, id)
    }

    fn p(x: f32, y: f32) -> Pos2 {
        Pos2::new(x, y)
    }

    #[test]
    fn consecutive_moves_on_same_node_coalesce_into_one_step() {
        let (mut map, id) = base();
        let start = map.get(id).unwrap().pos;
        let mut h = History::default();
        let t0 = Instant::now();
        assert!(h.run_at(
            Command::MoveNode {
                id,
                old_pos: start,
                new_pos: start + Vec2::new(5.0, 0.0)
            },
            &mut map,
            t0
        ));
        assert!(h.run_at(
            Command::MoveNode {
                id,
                old_pos: start + Vec2::new(5.0, 0.0),
                new_pos: start + Vec2::new(9.0, 0.0)
            },
            &mut map,
            t0 + Duration::from_millis(300)
        ));
        assert_eq!(map.get(id).unwrap().pos, start + Vec2::new(9.0, 0.0));

        // One step: undo jumps back to where the drag started.
        assert_eq!(h.undo(&mut map), Some("move node"));
        assert_eq!(map.get(id).unwrap().pos, start);

        // Redo lands at the final drag position.
        assert_eq!(h.redo(&mut map), Some("move node"));
        assert_eq!(map.get(id).unwrap().pos, start + Vec2::new(9.0, 0.0));
    }

    #[test]
    fn moves_beyond_the_window_open_new_steps() {
        let (mut map, id) = base();
        let start = map.get(id).unwrap().pos;
        let mut h = History::default();
        let t0 = Instant::now();
        assert!(h.run_at(
            Command::MoveNode {
                id,
                old_pos: start,
                new_pos: start + Vec2::new(5.0, 0.0)
            },
            &mut map,
            t0
        ));
        assert!(h.run_at(
            Command::MoveNode {
                id,
                old_pos: start + Vec2::new(5.0, 0.0),
                new_pos: start + Vec2::new(9.0, 0.0)
            },
            &mut map,
            t0 + COALESCE_WINDOW + Duration::from_millis(1)
        ));
        assert_eq!(h.undo(&mut map), Some("move node"));
        assert_eq!(map.get(id).unwrap().pos, start + Vec2::new(5.0, 0.0));
        assert_eq!(h.undo(&mut map), Some("move node"));
        assert_eq!(map.get(id).unwrap().pos, start);
    }

    #[test]
    fn different_node_or_kind_does_not_merge() {
        let (mut map, a) = base();
        let b = map.add_child(map.root).unwrap();
        let mut h = History::default();
        let t0 = Instant::now();

        // Different node, same kind.
        h.run_at(
            Command::MoveNode {
                id: a,
                old_pos: p(0.0, 0.0),
                new_pos: p(1.0, 0.0),
            },
            &mut map,
            t0,
        );
        h.run_at(
            Command::MoveNode {
                id: b,
                old_pos: p(0.0, 0.0),
                new_pos: p(2.0, 0.0),
            },
            &mut map,
            t0,
        );
        let mut undo_count = 0;
        while h.undo(&mut map).is_some() {
            undo_count += 1;
        }
        assert_eq!(undo_count, 2);

        // Same node, different kind.
        h.run_at(
            Command::MoveNode {
                id: a,
                old_pos: p(0.0, 0.0),
                new_pos: p(1.0, 0.0),
            },
            &mut map,
            t0,
        );
        h.run_at(
            Command::SetText {
                id: a,
                old: String::new(),
                new: "X".to_owned(),
            },
            &mut map,
            t0,
        );
        let mut undo_count = 0;
        while h.undo(&mut map).is_some() {
            undo_count += 1;
        }
        assert_eq!(undo_count, 2);
    }

    #[test]
    fn new_command_after_undo_invalidates_redo() {
        let (mut map, id) = base();
        let start = map.get(id).unwrap().pos;
        let mut h = History::default();
        let t0 = Instant::now();
        h.run_at(
            Command::MoveNode {
                id,
                old_pos: start,
                new_pos: start + Vec2::new(1.0, 1.0),
            },
            &mut map,
            t0,
        );
        assert!(h.undo(&mut map).is_some());
        assert!(h.can_redo());
        // A genuinely new command clears redo.
        h.run_at(
            Command::MoveNode {
                id,
                old_pos: start,
                new_pos: start + Vec2::new(2.0, 2.0),
            },
            &mut map,
            t0 + Duration::from_secs(10),
        );
        assert!(!h.can_redo());
        assert!(h.can_undo());
        assert_eq!(map.get(id).unwrap().pos, start + Vec2::new(2.0, 2.0));
    }

    #[test]
    fn undo_on_empty_history_is_none() {
        let (mut map, _) = base();
        let mut h = History::default();
        assert!(h.undo(&mut map).is_none());
        assert!(h.redo(&mut map).is_none());
    }

    /// L4 I2: ≥ 200 steps retained; beyond that the oldest is dropped.
    #[test]
    fn history_holds_at_least_200_steps_and_drops_the_oldest() {
        let mut map = MindMap::default();
        let mut ids = Vec::new();
        for _ in 0..250 {
            ids.push(map.add_child(map.root).unwrap());
        }
        let mut h = History::default();
        let t0 = Instant::now();
        for (i, &id) in ids.iter().enumerate() {
            let from = p(0.0, 0.0);
            let to = p(0.0, 0.0) + Vec2::new(i as f32, 0.0);
            h.run_at(
                Command::MoveNode {
                    id,
                    old_pos: from,
                    new_pos: to,
                },
                &mut map,
                t0 + Duration::from_secs(i as u64),
            );
        }
        let mut undone = 0;
        while h.undo(&mut map).is_some() {
            undone += 1;
        }
        assert_eq!(undone, 200);
        assert!(!h.can_undo());
        assert!(h.can_redo());

        let mut redone = 0;
        while h.redo(&mut map).is_some() {
            redone += 1;
        }
        assert_eq!(redone, 200);
    }

    #[test]
    fn batch_is_one_undo_step() {
        let (mut map, a) = base();
        let b = map.add_child(map.root).unwrap();
        let mut h = History::default();
        let batch = Command::Batch(vec![
            Command::SetText {
                id: a,
                old: String::new(),
                new: "A".to_owned(),
            },
            Command::SetText {
                id: b,
                old: String::new(),
                new: "B".to_owned(),
            },
        ]);
        assert!(h.run(batch, &mut map));
        assert_eq!(h.undo(&mut map), Some("batch edit"));
        assert_eq!(map.get(a).unwrap().text, "New topic");
        assert_eq!(map.get(b).unwrap().text, "New topic");
        assert_eq!(h.redo(&mut map), Some("batch edit"));
        assert_eq!(map.get(a).unwrap().text, "A");
        assert_eq!(map.get(b).unwrap().text, "B");
    }

    /// Undo then redo must return every touched node to the redo-final state.
    #[test]
    fn undo_redo_pair_is_lossless() {
        let (mut map, id) = base();
        let mut h = History::default();
        let t0 = Instant::now();
        h.run_at(
            Command::SetText {
                id,
                old: String::new(),
                new: "Renamed".to_owned(),
            },
            &mut map,
            t0,
        );
        h.run_at(
            Command::MoveNode {
                id,
                old_pos: p(0.0, 0.0),
                new_pos: p(3.0, 4.0),
            },
            &mut map,
            t0,
        );
        h.undo(&mut map);
        h.undo(&mut map);
        h.redo(&mut map);
        h.redo(&mut map);
        assert_eq!(map.get(id).unwrap().text, "Renamed");
    }
}
