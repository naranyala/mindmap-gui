//! Undo backbone (T1.3): every map mutation is a `Command` that can apply and
//! yield its exact inverse.

use egui::Pos2;

use super::{DeletedSubtree, MindMap, Node, NodeId};

#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    /// Creates a leaf node. `id` is pre-allocated by the model.
    AddNode {
        id: NodeId,
        parent: NodeId,
        after: Option<NodeId>,
        text: String,
        pos: Pos2,
    },
    /// Removes the node and its whole subtree.
    DeleteSubtree { id: NodeId },
    /// Internal: exact inverse of `DeleteSubtree`.
    RestoreSubtree {
        parent: NodeId,
        index: usize,
        nodes: Vec<Node>,
    },
    SetText {
        id: NodeId,
        old: String,
        new: String,
    },
    SetDetails {
        id: NodeId,
        old: String,
        new: String,
    },
    MoveNode {
        id: NodeId,
        old_pos: Pos2,
        new_pos: Pos2,
    },
    /// Applies several commands as one undo step (multi-select ops).
    Batch(Vec<Command>),
}

impl Command {
    /// The user-facing action this command performs. Used as the step label in
    /// the history, so "undid add node" means the popped step reverts an add.
    pub fn describe(&self) -> &'static str {
        match self {
            Self::AddNode { .. } => "add node",
            Self::DeleteSubtree { .. } => "delete node",
            Self::RestoreSubtree { .. } => "restore node",
            Self::SetText { .. } => "rename node",
            Self::SetDetails { .. } => "edit node details",
            Self::MoveNode { .. } => "move node",
            Self::Batch(..) => "batch edit",
        }
    }

    /// Applies `self` to `map` and returns the inverse command that undoes
    /// exactly this application, or `None` if it was a no-op or invalid.
    pub fn run(&self, map: &mut MindMap) -> Option<Command> {
        match self {
            Self::AddNode {
                id,
                parent,
                after,
                text,
                pos,
            } => {
                let node = Node {
                    id: *id,
                    parent: Some(*parent),
                    children: Vec::new(),
                    pos: *pos,
                    text: text.clone(),
                    details: String::new(),
                };
                if !map.insert_with(node, *after) {
                    return None;
                }
                Some(Self::DeleteSubtree { id: *id })
            }
            Self::DeleteSubtree { id } => {
                let DeletedSubtree {
                    parent,
                    index,
                    nodes,
                } = map.remove_subtree(*id)?;
                Some(Self::RestoreSubtree {
                    parent,
                    index,
                    nodes,
                })
            }
            Self::RestoreSubtree {
                parent,
                index,
                nodes,
            } => {
                let top = nodes.first()?;
                let top_id = top.id;
                if !map.restore_subtree(*parent, *index, nodes) {
                    return None;
                }
                Some(Self::DeleteSubtree { id: top_id })
            }
            Self::SetText { id, new, .. } => {
                let prev = map.get(*id).map(|n| n.text.clone())?;
                if prev == *new || !map.set_text(*id, new.clone()) {
                    return None;
                }
                Some(Self::SetText {
                    id: *id,
                    old: new.clone(),
                    new: prev,
                })
            }
            Self::SetDetails { id, new, .. } => {
                let prev = map.get(*id).map(|n| n.details.clone())?;
                if prev == *new || !map.set_details(*id, new.clone()) {
                    return None;
                }
                Some(Self::SetDetails {
                    id: *id,
                    old: new.clone(),
                    new: prev,
                })
            }
            Self::MoveNode { id, new_pos, .. } => {
                let prev = map.get(*id).map(|n| n.pos)?;
                if prev == *new_pos {
                    return None;
                }
                map.get_mut(*id)?.pos = *new_pos;
                Some(Self::MoveNode {
                    id: *id,
                    old_pos: *new_pos,
                    new_pos: prev,
                })
            }
            Self::Batch(cmds) => {
                let mut inverses = Vec::with_capacity(cmds.len());
                for cmd in cmds {
                    if let Some(inverse) = cmd.run(map) {
                        inverses.push(inverse);
                    }
                }
                match inverses.len() {
                    0 => None,
                    1 => Some(inverses.into_iter().next().expect("len 1")),
                    _ => {
                        inverses.reverse();
                        Some(Self::Batch(inverses))
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{load_from_str, save_to_string};

    #[test]
    fn add_node_undo_redo_recreates_with_same_id() {
        let mut map = MindMap::default();
        let root = map.root;
        let (id, cmd) = map.add_child_cmd(root).unwrap();

        let inverse = cmd.run(&mut map).unwrap();
        assert!(map.contains(id));
        assert_eq!(map.get(root).unwrap().children, vec![id]);

        let forward = inverse.run(&mut map).unwrap();
        assert!(!map.contains(id));
        assert_eq!(map.get(root).unwrap().children, vec![]);

        assert!(forward.run(&mut map).is_some());
        assert!(map.contains(id));
        assert_eq!(map.get(root).unwrap().children, vec![id]);
        assert_eq!(map.get(id).unwrap().text, "New topic");
    }

    #[test]
    fn delete_subtree_undo_restores_slot_in_parent_children() {
        let mut map = MindMap::default();
        let root = map.root;
        let a = map.add_child(root).unwrap();
        let b = map.add_child(root).unwrap();
        let c = map.add_child(root).unwrap();

        let cmd = Command::DeleteSubtree { id: b };
        let inverse = cmd.run(&mut map).unwrap();
        assert_eq!(map.get(root).unwrap().children, vec![a, c]);

        let forward = inverse.run(&mut map).unwrap();
        assert_eq!(map.get(root).unwrap().children, vec![a, b, c]);

        forward.run(&mut map).unwrap();
        assert_eq!(map.get(root).unwrap().children, vec![a, c]);
    }

    #[test]
    fn delete_subtree_refuses_root() {
        let mut map = MindMap::default();
        assert!(
            Command::DeleteSubtree { id: map.root }
                .run(&mut map)
                .is_none()
        );
    }

    #[test]
    fn set_text_inverse_restores_previous_text() {
        let mut map = MindMap::default();
        let root = map.root;
        let cmd = Command::SetText {
            id: root,
            old: "stale".to_owned(),
            new: "Ideas".to_owned(),
        };
        let inverse = cmd.run(&mut map).unwrap();
        assert_eq!(map.get(root).unwrap().text, "Ideas");

        // The inverse uses the text that was actually there, not `old`.
        assert_eq!(
            inverse,
            Command::SetText {
                id: root,
                old: "Ideas".to_owned(),
                new: "Central Topic".to_owned(),
            }
        );

        inverse.run(&mut map).unwrap();
        assert_eq!(map.get(root).unwrap().text, "Central Topic");
    }

    #[test]
    fn set_text_noop_when_unchanged() {
        let mut map = MindMap::default();
        let cmd = Command::SetText {
            id: map.root,
            old: "Central Topic".to_owned(),
            new: "Central Topic".to_owned(),
        };
        assert!(cmd.run(&mut map).is_none());
    }

    #[test]
    fn set_details_undo_redo_roundtrip() {
        let mut map = MindMap::default();
        let root = map.root;
        let cmd = Command::SetDetails {
            id: root,
            old: String::new(),
            new: "A longer explanation".to_owned(),
        };
        let inverse = cmd.run(&mut map).expect("details should change");
        assert_eq!(map.get(root).unwrap().details, "A longer explanation");

        let forward = inverse.run(&mut map).expect("inverse should apply");
        assert!(map.get(root).unwrap().details.is_empty());
        assert!(forward.run(&mut map).is_some());
        assert_eq!(map.get(root).unwrap().details, "A longer explanation");
    }

    #[test]
    fn move_node_inverse_swaps_positions() {
        let mut map = MindMap::default();
        let root = map.root;
        let a = map.get(root).unwrap().pos; // actual pre-state position
        let b = Pos2::new(50.0, 30.0);
        let cmd = Command::MoveNode {
            id: root,
            old_pos: a,
            new_pos: b,
        };
        let inverse = cmd.run(&mut map).unwrap();
        assert_eq!(map.get(root).unwrap().pos, b);
        assert_eq!(
            inverse,
            Command::MoveNode {
                id: root,
                old_pos: b,
                new_pos: a
            }
        );

        inverse.run(&mut map).unwrap();
        assert_eq!(map.get(root).unwrap().pos, a);
    }

    #[test]
    fn batch_undoes_and_redoes_atomically() {
        let mut map = MindMap::default();
        let root = map.root;
        let a = map.add_child(root).unwrap();
        let b = map.add_child(root).unwrap();
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
        let inverse = batch.run(&mut map).unwrap();
        assert_eq!(map.get(a).unwrap().text, "A");
        assert_eq!(map.get(b).unwrap().text, "B");
        assert!(matches!(inverse, Command::Batch(_)));

        inverse.run(&mut map).unwrap();
        assert_eq!(map.get(a).unwrap().text, "New topic");
        assert_eq!(map.get(b).unwrap().text, "New topic");

        assert!(batch.run(&mut map).is_some());
        assert_eq!(map.get(a).unwrap().text, "A");
        assert_eq!(map.get(b).unwrap().text, "B");
    }

    /// Running a command and then its inverse returns the map to a state that
    /// is semantically identical (ids may drift via the monotonic `next_id`).
    #[test]
    fn command_plus_inverse_roundtrips_to_identical_state() {
        fn semantic(map: &MindMap) -> String {
            map.iter().map(|n| format!("{:?}", n)).collect()
        }

        let mut map = MindMap::default();
        let root = map.root;
        map.add_child(root).unwrap();
        let before = semantic(&map);

        let (id, cmd) = map.add_child_cmd(root).unwrap();
        let mut step = cmd;
        for _ in 0..4 {
            let Some(next) = step.run(&mut map) else {
                break;
            };
            step = next;
        }
        assert!(
            !map.contains(id),
            "even number of runs should remove the node"
        );

        assert_eq!(semantic(&map), before);
    }

    #[test]
    fn inverse_chain_through_load() {
        let mut map = MindMap::default();
        map.add_child(map.root).unwrap();
        let json = save_to_string(&map).unwrap();
        let mut loaded = load_from_str(&json).unwrap();
        let (id, cmd) = loaded.add_child_cmd(loaded.root).unwrap();
        let inverse = cmd.run(&mut loaded).unwrap();
        assert!(loaded.contains(id));
        inverse.run(&mut loaded).unwrap();
        assert!(!loaded.contains(id));
        assert_eq!(loaded.len(), 2);
    }
}
