use std::collections::HashSet;
use std::path::Path;

use egui::{Pos2, Rect, Vec2};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub u64);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    pub pos: Pos2,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MindMap {
    pub root: NodeId,
    pub nodes: Vec<Node>,
    next_id: u64,
}

const SIBLING_GAP: f32 = 84.0;
const CHILD_GAP: f32 = 76.0;
const CHILD_DX: f32 = 280.0;

impl Default for MindMap {
    fn default() -> Self {
        let root = NodeId(1);
        Self {
            root,
            nodes: vec![Node {
                id: root,
                parent: None,
                children: Vec::new(),
                pos: Pos2::ZERO,
                text: "Central Topic".to_owned(),
            }],
            next_id: 2,
        }
    }
}

impl MindMap {
    pub fn get(&self, id: NodeId) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        self.nodes.iter_mut().find(|n| n.id == id)
    }

    pub fn contains(&self, id: NodeId) -> bool {
        self.get(id).is_some()
    }

    pub fn set_text(&mut self, id: NodeId, text: String) -> bool {
        match self.get_mut(id) {
            Some(node) if node.text != text => {
                node.text = text;
                true
            }
            _ => false,
        }
    }

    pub fn add_child(&mut self, parent: NodeId) -> Option<NodeId> {
        let index = self.get(parent)?.children.len();
        let pos = self.get(parent)?.pos + Vec2::new(CHILD_DX, index as f32 * CHILD_GAP);
        self.insert_node(parent, None, "New topic", pos)
    }

    pub fn add_sibling(&mut self, id: NodeId) -> Option<NodeId> {
        if id == self.root {
            return self.add_child(self.root);
        }
        let parent = self.get(id)?.parent?;
        let pos = self.get(id)?.pos + Vec2::new(0.0, SIBLING_GAP);
        self.insert_node(parent, Some(id), "New topic", pos)
    }

    fn insert_node(
        &mut self,
        parent: NodeId,
        after: Option<NodeId>,
        text: &str,
        pos: Pos2,
    ) -> Option<NodeId> {
        if !self.contains(parent) {
            return None;
        }
        let id = NodeId(self.next_id);
        self.next_id += 1;
        let node = Node {
            id,
            parent: Some(parent),
            children: Vec::new(),
            pos,
            text: text.to_owned(),
        };
        {
            let parent_node = self.get_mut(parent)?;
            let index = match after.and_then(|a| parent_node.children.iter().position(|&c| c == a))
            {
                Some(i) => i + 1,
                None => parent_node.children.len(),
            };
            parent_node.children.insert(index, id);
        }
        self.nodes.push(node);
        Some(id)
    }

    /// Deletes the node and all of its descendants. Refuses to delete the root.
    pub fn delete_subtree(&mut self, id: NodeId) -> bool {
        if id == self.root || !self.contains(id) {
            return false;
        }
        let parent = self.get(id).and_then(|n| n.parent);
        if let Some(pid) = parent
            && let Some(p) = self.get_mut(pid)
        {
            p.children.retain(|&c| c != id);
        }
        let mut doomed = HashSet::new();
        let mut stack = vec![id];
        while let Some(cur) = stack.pop() {
            if doomed.insert(cur)
                && let Some(node) = self.get(cur)
            {
                stack.extend(node.children.iter().copied());
            }
        }
        self.nodes.retain(|n| !doomed.contains(&n.id));
        true
    }

    /// Bounding box around all nodes, padded. Used to frame the view.
    pub fn bounds(&self) -> Option<Rect> {
        let mut iter = self.nodes.iter();
        let first = iter.next()?;
        let mut min = first.pos;
        let mut max = first.pos;
        for node in iter {
            min = min.min(node.pos);
            max = max.max(node.pos);
        }
        Some(Rect::from_min_max(min, max).expand(180.0))
    }

    /// Repairs a loaded map: drops unknown/dangling references and unreachable
    /// nodes, and rebuilds parent links from the children lists.
    pub fn restructure(&mut self) -> bool {
        if !self.contains(self.root) {
            return false;
        }
        let ids: HashSet<NodeId> = self.nodes.iter().map(|n| n.id).collect();
        for node in &mut self.nodes {
            node.children.retain(|c| ids.contains(c) && *c != node.id);
        }

        let mut reachable = HashSet::new();
        let mut stack = vec![self.root];
        while let Some(cur) = stack.pop() {
            if !reachable.insert(cur) {
                continue;
            }
            if let Some(node) = self.get(cur) {
                stack.extend(node.children.iter().copied());
            }
        }
        self.nodes.retain(|n| reachable.contains(&n.id));

        let children: Vec<(NodeId, Vec<NodeId>)> = self
            .nodes
            .iter()
            .map(|n| (n.id, n.children.clone()))
            .collect();
        for node in &mut self.nodes {
            node.parent = if node.id == self.root {
                None
            } else {
                children
                    .iter()
                    .find(|(_, kids)| kids.contains(&node.id))
                    .map(|(pid, _)| *pid)
            };
        }
        true
    }
}

// ----------------------------------------------------------------------------

const FILE_APP: &str = "mindmap-gui";
const FILE_VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
struct FileDoc {
    app: String,
    version: u32,
    map: MindMap,
}

pub fn save_to_string(map: &MindMap) -> Result<String, String> {
    let doc = FileDoc {
        app: FILE_APP.to_owned(),
        version: FILE_VERSION,
        map: map.clone(),
    };
    serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())
}

pub fn load_from_str(json: &str) -> Result<MindMap, String> {
    let doc: FileDoc = serde_json::from_str(json).map_err(|e| e.to_string())?;
    if doc.version > FILE_VERSION {
        return Err(format!(
            "file version {} is newer than supported version {}",
            doc.version, FILE_VERSION
        ));
    }
    let mut map = doc.map;
    if !map.restructure() {
        return Err("file has no valid root node".to_owned());
    }
    Ok(map)
}

pub fn save_file(map: &MindMap, path: &Path) -> Result<(), String> {
    let json = save_to_string(map)?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

pub fn load_file(path: &Path) -> Result<MindMap, String> {
    let json = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    load_from_str(&json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_map_has_root() {
        let map = MindMap::default();
        assert!(map.contains(map.root));
        assert_eq!(map.nodes.len(), 1);
        assert!(map.get(map.root).unwrap().parent.is_none());
    }

    #[test]
    fn add_child_links_both_ways() {
        let mut map = MindMap::default();
        let root = map.root;
        let child = map.add_child(root).unwrap();
        let node = map.get(child).unwrap();
        assert_eq!(node.parent, Some(root));
        assert_eq!(map.get(root).unwrap().children, vec![child]);
        assert_eq!(node.pos.x, map.get(root).unwrap().pos.x + CHILD_DX);
    }

    #[test]
    fn add_sibling_inserts_after_and_root_sibling_is_child() {
        let mut map = MindMap::default();
        let root = map.root;
        let a = map.add_child(root).unwrap();
        let b = map.add_child(root).unwrap();
        let c = map.add_sibling(a).unwrap();
        assert_eq!(map.get(root).unwrap().children, vec![a, c, b]);

        // Sibling of the root becomes a child of the root.
        let r = map.add_sibling(root).unwrap();
        assert_eq!(map.get(root).unwrap().children, vec![a, c, b, r]);
        assert_eq!(map.get(r).unwrap().parent, Some(root));
    }

    #[test]
    fn delete_subtree_removes_descendants() {
        let mut map = MindMap::default();
        let root = map.root;
        let a = map.add_child(root).unwrap();
        let a1 = map.add_child(a).unwrap();
        let a2 = map.add_child(a1).unwrap();
        let b = map.add_child(root).unwrap();

        assert!(map.delete_subtree(a));
        assert!(!map.contains(a) && !map.contains(a1) && !map.contains(a2));
        assert!(map.contains(b));
        assert_eq!(map.get(root).unwrap().children, vec![b]);
    }

    #[test]
    fn cannot_delete_root() {
        let mut map = MindMap::default();
        let root = map.root;
        assert!(!map.delete_subtree(root));
        assert!(map.contains(root));
    }

    #[test]
    fn set_text_only_marks_changed_text() {
        let mut map = MindMap::default();
        let root = map.root;
        assert!(map.set_text(root, "Ideas".to_owned()));
        assert!(!map.set_text(root, "Ideas".to_owned()));
        assert_eq!(map.get(root).unwrap().text, "Ideas");
    }

    #[test]
    fn save_load_roundtrip() {
        let mut map = MindMap::default();
        let root = map.root;
        let a = map.add_child(root).unwrap();
        map.set_text(a, "Research".to_owned());

        let json = save_to_string(&map).unwrap();
        let loaded = load_from_str(&json).unwrap();

        assert_eq!(loaded.root, map.root);
        assert_eq!(loaded.nodes.len(), map.nodes.len());
        let node = loaded.get(a).unwrap();
        assert_eq!(node.text, "Research");
        assert_eq!(node.parent, Some(root));
        assert!(loaded.get(root).unwrap().children.contains(&a));
    }

    #[test]
    fn restructure_drops_dangling_and_unreachable() {
        let mut map = MindMap::default();
        let root = map.root;
        let a = map.add_child(root).unwrap();

        // Dangling child reference.
        map.get_mut(a).unwrap().children.push(NodeId(999));
        // Unreachable node.
        map.nodes.push(Node {
            id: NodeId(500),
            parent: Some(NodeId(998)),
            children: vec![],
            pos: Pos2::new(10.0, 10.0),
            text: "orphan".to_owned(),
        });

        assert!(map.restructure());
        assert!(!map.contains(NodeId(999)));
        assert!(!map.contains(NodeId(500)));
        assert!(map.contains(a));
    }

    #[test]
    fn restructure_rejects_missing_root() {
        let mut map = MindMap::default();
        map.root = NodeId(4242);
        assert!(!map.restructure());
    }
}
