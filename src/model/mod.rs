pub mod command;
pub mod history;

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::path::Path;

use egui::{Pos2, Rect, Vec2};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub u64);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    pub pos: Pos2,
    pub text: String,
    #[serde(default)]
    pub details: String,
}

const SIBLING_GAP: f32 = 84.0;
const CHILD_GAP: f32 = 76.0;
const CHILD_DX: f32 = 280.0;

/// Snapshot of a removed subtree: its nodes in stable serialize order, plus
/// where the top node was attached to its parent. Carries everything needed
/// to restore the subtree exactly. (T1.3)
#[derive(Clone, Debug)]
pub struct DeletedSubtree {
    pub parent: NodeId,
    pub index: usize,
    pub nodes: Vec<Node>,
}

/// Hash-indexed model (T1.1): `get`/`get_mut` are O(1); `order` keeps the
/// stable paint/serialize order the old `Vec<Node>` gave for free.
///
/// Serializes exactly like the v1 `Vec`-based format, so existing files keep
/// loading unchanged.
#[derive(Clone, Debug)]
pub struct MindMap {
    pub root: NodeId,
    nodes: HashMap<NodeId, Node>,
    order: Vec<NodeId>,
    next_id: u64,
}

/// Maps that ship with the editor as starting points for new ideas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuiltInMap {
    Blank,
    ProjectPlan,
    StudyPlan,
    MeetingAgenda,
}

impl BuiltInMap {
    pub const ALL: [Self; 4] = [
        Self::Blank,
        Self::ProjectPlan,
        Self::StudyPlan,
        Self::MeetingAgenda,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Blank => "Blank map",
            Self::ProjectPlan => "Project plan",
            Self::StudyPlan => "Study plan",
            Self::MeetingAgenda => "Meeting agenda",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Blank => "Start with a single central topic",
            Self::ProjectPlan => "Shape a project from goals to next steps",
            Self::StudyPlan => "Organize a subject into a focused study path",
            Self::MeetingAgenda => "Prepare a meeting with decisions and follow-ups",
        }
    }

    pub fn map(self) -> MindMap {
        match self {
            Self::Blank => MindMap::default(),
            Self::ProjectPlan => project_plan(),
            Self::StudyPlan => study_plan(),
            Self::MeetingAgenda => meeting_agenda(),
        }
    }
}

fn map_with_root(text: &str) -> MindMap {
    let mut map = MindMap::default();
    map.set_text(map.root, text.to_owned());
    map
}

fn add_branch(map: &mut MindMap, parent: NodeId, text: &str, y: f32) -> NodeId {
    let id = map.add_child(parent).expect("built-in map parent exists");
    let node = map.get_mut(id).expect("built-in map node exists");
    node.text = text.to_owned();
    node.pos.y = y;
    id
}

fn project_plan() -> MindMap {
    let mut map = map_with_root("Project plan");
    let root = map.root;
    let goals = add_branch(&mut map, root, "Goals", -150.0);
    add_branch(&mut map, goals, "Define success", -210.0);
    add_branch(&mut map, goals, "Set milestones", -130.0);
    let work = add_branch(&mut map, root, "Work", 0.0);
    add_branch(&mut map, work, "Research", -60.0);
    add_branch(&mut map, work, "Build", 20.0);
    add_branch(&mut map, work, "Review", 100.0);
    add_branch(&mut map, root, "Next steps", 150.0);
    map
}

fn study_plan() -> MindMap {
    let mut map = map_with_root("Study plan");
    let root = map.root;
    let concepts = add_branch(&mut map, root, "Core concepts", -120.0);
    add_branch(&mut map, concepts, "Definitions", -180.0);
    add_branch(&mut map, concepts, "Examples", -100.0);
    let practice = add_branch(&mut map, root, "Practice", 40.0);
    add_branch(&mut map, practice, "Exercises", -20.0);
    add_branch(&mut map, practice, "Flash cards", 60.0);
    add_branch(&mut map, root, "Review", 180.0);
    map
}

fn meeting_agenda() -> MindMap {
    let mut map = map_with_root("Meeting agenda");
    let root = map.root;
    add_branch(&mut map, root, "Context", -150.0);
    let decisions = add_branch(&mut map, root, "Decisions", 0.0);
    add_branch(&mut map, decisions, "Decision 1", -60.0);
    add_branch(&mut map, decisions, "Decision 2", 20.0);
    add_branch(&mut map, root, "Action items", 150.0);
    map
}

impl Default for MindMap {
    fn default() -> Self {
        let root = NodeId(1);
        let node = Node {
            id: root,
            parent: None,
            children: Vec::new(),
            pos: Pos2::ZERO,
            text: "Central Topic".to_owned(),
            details: String::new(),
        };
        Self {
            root,
            nodes: HashMap::from([(root, node)]),
            order: vec![root],
            next_id: 2,
        }
    }
}

impl MindMap {
    pub fn get(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(&id)
    }

    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        self.nodes.get_mut(&id)
    }

    pub fn contains(&self, id: NodeId) -> bool {
        self.nodes.contains_key(&id)
    }

    /// All nodes in stable paint/serialize order.
    pub fn iter(&self) -> impl Iterator<Item = &Node> {
        self.order.iter().filter_map(|id| self.nodes.get(id))
    }

    pub fn len(&self) -> usize {
        self.order.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
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

    pub fn set_details(&mut self, id: NodeId, details: String) -> bool {
        match self.get_mut(id) {
            Some(node) if node.details != details => {
                node.details = details;
                true
            }
            _ => false,
        }
    }

    // ------------------------------------------------------- planning (T1.3)

    fn alloc_id(&mut self) -> NodeId {
        let id = NodeId(self.next_id);
        self.next_id += 1;
        id
    }

    /// Plans a child-add as a command without mutating the map.
    pub fn add_child_cmd(&mut self, parent: NodeId) -> Option<(NodeId, command::Command)> {
        if !self.contains(parent) {
            return None;
        }
        let index = self.get(parent)?.children.len();
        let pos = self.get(parent)?.pos + Vec2::new(CHILD_DX, index as f32 * CHILD_GAP);
        let id = self.alloc_id();
        Some((
            id,
            command::Command::AddNode {
                id,
                parent,
                after: None,
                text: "New topic".to_owned(),
                pos,
            },
        ))
    }

    /// Plans a sibling-add as a command. Sibling of the root = child of root.
    pub fn add_sibling_cmd(&mut self, id: NodeId) -> Option<(NodeId, command::Command)> {
        if id == self.root {
            return self.add_child_cmd(self.root);
        }
        let parent = self.get(id)?.parent?;
        let pos = self.get(id)?.pos + Vec2::new(0.0, SIBLING_GAP);
        let new_id = self.alloc_id();
        Some((
            new_id,
            command::Command::AddNode {
                id: new_id,
                parent,
                after: Some(id),
                text: "New topic".to_owned(),
                pos,
            },
        ))
    }

    // -------------------------------------------------- structure primitives

    /// Inserts a fully specified node. Used by `Command::AddNode` (T1.3).
    pub fn insert_with(&mut self, node: Node, after: Option<NodeId>) -> bool {
        let id = node.id;
        if self.nodes.contains_key(&id) {
            return false;
        }
        let Some(parent) = node.parent else {
            return false;
        };
        let Some(pnode) = self.nodes.get_mut(&parent) else {
            return false;
        };
        let index = match after.and_then(|a| pnode.children.iter().position(|&c| c == a)) {
            Some(i) => i + 1,
            None => pnode.children.len(),
        };
        pnode.children.insert(index, id);
        if self.next_id <= id.0 {
            self.next_id = id.0 + 1;
        }
        self.nodes.insert(id, node);
        self.order.push(id);
        true
    }

    /// Convenience wrappers kept for direct (history-free) use and tests.
    // Used from tests and reserved for T1.4's placement work; the app itself
    // mutates through `Command`s.
    #[allow(dead_code)]
    pub fn add_child(&mut self, parent: NodeId) -> Option<NodeId> {
        let (id, cmd) = self.add_child_cmd(parent)?;
        cmd.run(self);
        Some(id)
    }

    #[allow(dead_code)]
    pub fn add_sibling(&mut self, id: NodeId) -> Option<NodeId> {
        let (new_id, cmd) = self.add_sibling_cmd(id)?;
        cmd.run(self);
        Some(new_id)
    }

    /// Removes `id` and all descendants, capturing a restorable snapshot.
    pub fn remove_subtree(&mut self, id: NodeId) -> Option<DeletedSubtree> {
        if id == self.root || !self.contains(id) {
            return None;
        }
        let parent = self.get(id)?.parent?;
        let index = self.get(parent)?.children.iter().position(|&c| c == id)?;
        self.get_mut(parent)?.children.remove(index);

        let mut doomed = HashSet::new();
        let mut stack = vec![id];
        while let Some(cur) = stack.pop() {
            if doomed.insert(cur)
                && let Some(node) = self.get(cur)
            {
                stack.extend(node.children.iter().copied());
            }
        }
        let nodes: Vec<Node> = self
            .order
            .iter()
            .filter(|i| doomed.contains(i))
            .map(|i| self.nodes.get(i).expect("ordered node exists").clone())
            .collect();
        for d in &doomed {
            self.nodes.remove(d);
        }
        self.order.retain(|i| !doomed.contains(i));
        Some(DeletedSubtree {
            parent,
            index,
            nodes,
        })
    }

    /// Re-inserts a snapshot from [`Self::remove_subtree`] at its old slot.
    pub fn restore_subtree(&mut self, parent: NodeId, index: usize, nodes: &[Node]) -> bool {
        if !self.nodes.contains_key(&parent) || nodes.is_empty() {
            return false;
        }
        if nodes.iter().any(|n| self.nodes.contains_key(&n.id)) {
            return false;
        }
        let children_len = self.get(parent).map(|n| n.children.len()).unwrap_or(0);
        let attach = index.min(children_len);
        let top_id = nodes[0].id;
        let Some(pnode) = self.get_mut(parent) else {
            return false;
        };
        pnode.children.insert(attach, top_id);
        let mut max_seen = self.next_id.saturating_sub(1);
        for node in nodes {
            max_seen = max_seen.max(node.id.0);
            self.nodes.insert(node.id, node.clone());
            self.order.push(node.id);
        }
        self.next_id = self.next_id.max(max_seen + 1);
        true
    }

    /// Deletes the node and all of its descendants. Refuses to delete the root.
    #[allow(dead_code)]
    pub fn delete_subtree(&mut self, id: NodeId) -> bool {
        self.remove_subtree(id).is_some()
    }

    /// Bounding box around all nodes, padded. Used to frame the view.
    pub fn bounds(&self) -> Option<Rect> {
        let mut iter = self.nodes.values();
        let first = iter.next()?;
        let mut min = first.pos;
        let mut max = first.pos;
        for node in iter {
            min = min.min(node.pos);
            max = max.max(node.pos);
        }
        Some(Rect::from_min_max(min, max).expand(180.0))
    }

    /// Bounding box around a node and all of its descendants.
    pub fn subtree_bounds(&self, root: NodeId) -> Option<Rect> {
        let mut stack = vec![root];
        let mut positions = Vec::new();
        while let Some(id) = stack.pop() {
            let node = self.get(id)?;
            positions.push(node.pos);
            stack.extend(node.children.iter().copied());
        }
        let first = *positions.first()?;
        let (mut min, mut max) = (first, first);
        for pos in positions.into_iter().skip(1) {
            min = min.min(pos);
            max = max.max(pos);
        }
        Some(Rect::from_min_max(min, max).expand(180.0))
    }

    /// Repairs a loaded map: drops unknown/dangling references and unreachable
    /// nodes, and rebuilds parent links from the children lists.
    pub fn restructure(&mut self) -> bool {
        if !self.nodes.contains_key(&self.root) {
            return false;
        }
        let ids: HashSet<NodeId> = self.nodes.keys().copied().collect();
        for node in self.nodes.values_mut() {
            node.children.retain(|c| ids.contains(c) && *c != node.id);
        }

        let mut reachable = HashSet::new();
        let mut stack = vec![self.root];
        while let Some(cur) = stack.pop() {
            if !reachable.insert(cur) {
                continue;
            }
            if let Some(node) = self.nodes.get(&cur) {
                stack.extend(node.children.iter().copied());
            }
        }
        self.order.retain(|i| reachable.contains(i));
        self.nodes.retain(|i, _| reachable.contains(i));

        let mut parent_of: HashMap<NodeId, NodeId> = HashMap::new();
        for node in self.nodes.values() {
            for &c in &node.children {
                parent_of.insert(c, node.id);
            }
        }
        for node in self.nodes.values_mut() {
            node.parent = if node.id == self.root {
                None
            } else {
                parent_of.get(&node.id).copied()
            };
        }

        let max = self.nodes.keys().map(|i| i.0).max().unwrap_or(self.root.0);
        self.next_id = self.next_id.max(max + 1);
        true
    }

    /// Reassembles a map from its deserialized parts. Used by (de)serialization.
    fn from_parts(root: NodeId, nodes: Vec<Node>, next_id: u64) -> Self {
        let order: Vec<NodeId> = nodes.iter().map(|n| n.id).collect();
        let mut map = HashMap::with_capacity(nodes.len());
        for node in nodes {
            map.insert(node.id, node);
        }
        let max = map.keys().map(|i| i.0).max().unwrap_or(root.0);
        Self {
            root,
            nodes: map,
            order,
            next_id: next_id.max(max + 1),
        }
    }
}

// ----------------------------------------------------------------------------
// Serialization — byte-compatible with the v1 Vec-based format.

#[derive(Serialize, Deserialize)]
struct MindMapSer {
    root: NodeId,
    nodes: Vec<Node>,
    next_id: u64,
}

impl From<&MindMap> for MindMapSer {
    fn from(map: &MindMap) -> Self {
        Self {
            root: map.root,
            nodes: map.iter().cloned().collect(),
            next_id: map.next_id,
        }
    }
}

impl Serialize for MindMap {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        MindMapSer::from(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for MindMap {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let ser = MindMapSer::deserialize(deserializer)?;
        Ok(Self::from_parts(ser.root, ser.nodes, ser.next_id))
    }
}

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

/// Exports a readable, self-contained SVG image. When `branch_root` is set,
/// only that node and its descendants are included.
pub fn save_svg(map: &MindMap, path: &Path, branch_root: Option<NodeId>) -> Result<(), String> {
    let visible: HashSet<NodeId> = if let Some(root) = branch_root {
        let mut ids = HashSet::new();
        let mut stack = vec![root];
        while let Some(id) = stack.pop() {
            if ids.insert(id)
                && let Some(node) = map.get(id)
            {
                stack.extend(node.children.iter().copied());
            }
        }
        ids
    } else {
        map.iter().map(|node| node.id).collect()
    };
    let bounds = branch_root
        .and_then(|root| map.subtree_bounds(root))
        .or_else(|| map.bounds())
        .ok_or_else(|| "cannot export an empty mindmap".to_owned())?;
    let width = bounds.width().max(240.0);
    let height = bounds.height().max(160.0);
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width:.0}\" height=\"{height:.0}\" viewBox=\"0 0 {width:.0} {height:.0}\">\n"
    );
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"#ffffff\"/>\n");

    for node in map.iter().filter(|node| visible.contains(&node.id)) {
        for child_id in &node.children {
            if let Some(child) = map.get(*child_id)
                && visible.contains(child_id)
            {
                let x1 = node.pos.x - bounds.min.x;
                let y1 = node.pos.y - bounds.min.y;
                let x2 = child.pos.x - bounds.min.x;
                let y2 = child.pos.y - bounds.min.y;
                writeln!(
                    svg,
                    "<path d=\"M {x1:.1} {y1:.1} C {mx:.1} {y1:.1}, {mx:.1} {y2:.1}, {x2:.1} {y2:.1}\" fill=\"none\" stroke=\"#9aa4b2\" stroke-width=\"2\"/>",
                    mx = (x1 + x2) / 2.0
                )
                .expect("writing SVG to a String cannot fail");
            }
        }
    }

    for node in map.iter().filter(|node| visible.contains(&node.id)) {
        let x = node.pos.x - bounds.min.x - 70.0;
        let y = node.pos.y - bounds.min.y - 28.0;
        let height = 56.0 + node.details.lines().count().max(1) as f32 * 17.0;
        writeln!(
            svg,
            "<rect x=\"{x:.1}\" y=\"{y:.1}\" width=\"140\" height=\"{height:.1}\" rx=\"10\" fill=\"#eef3ff\" stroke=\"#5874b8\" stroke-width=\"2\"/>"
        )
        .expect("writing SVG to a String cannot fail");
        let tx = x + 12.0;
        writeln!(
            svg,
            "<text x=\"{tx:.1}\" y=\"{:.1}\" font-family=\"sans-serif\" font-size=\"15\" font-weight=\"600\" fill=\"#172033\">{}</text>",
            y + 23.0,
            svg_escape(&node.text)
        )
        .expect("writing SVG to a String cannot fail");
        for (index, line) in node.details.lines().enumerate() {
            writeln!(
                svg,
                "<text x=\"{tx:.1}\" y=\"{:.1}\" font-family=\"sans-serif\" font-size=\"11\" fill=\"#4f5d75\">{}</text>",
                y + 43.0 + index as f32 * 17.0,
                svg_escape(line)
            )
            .expect("writing SVG to a String cannot fail");
        }
    }
    svg.push_str("</svg>\n");
    std::fs::write(path, svg).map_err(|e| e.to_string())
}

fn svg_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn default_map_has_root() {
        let map = MindMap::default();
        assert!(map.contains(map.root));
        assert_eq!(map.len(), 1);
        assert!(map.get(map.root).unwrap().parent.is_none());
    }

    #[test]
    fn built_in_maps_are_valid_and_non_empty() {
        for template in BuiltInMap::ALL {
            let map = template.map();
            assert!(map.contains(map.root));
            assert!(!map.is_empty());
            assert!(map.get(map.root).unwrap().parent.is_none());
            assert!(!map.get(map.root).unwrap().text.is_empty());
        }
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
        map.set_details(a, "Sources, risks, and follow-up questions".to_owned());

        let json = save_to_string(&map).unwrap();
        let loaded = load_from_str(&json).unwrap();

        assert_eq!(loaded.root, map.root);
        assert_eq!(loaded.len(), map.len());
        let node = loaded.get(a).unwrap();
        assert_eq!(node.text, "Research");
        assert_eq!(node.details, "Sources, risks, and follow-up questions");
        assert_eq!(node.parent, Some(root));
        assert!(loaded.get(root).unwrap().children.contains(&a));
    }

    /// The stable serialize order (`order`) must match the old Vec behavior.
    #[test]
    fn serialization_preserves_insertion_order() {
        let mut map = MindMap::default();
        let root = map.root;
        let a = map.add_child(root).unwrap();
        map.add_child(root).unwrap();
        let c = map.add_sibling(a).unwrap();

        let before: Vec<NodeId> = map.iter().map(|n| n.id).collect();
        let loaded = load_from_str(&save_to_string(&map).unwrap()).unwrap();
        let after: Vec<NodeId> = loaded.iter().map(|n| n.id).collect();
        assert_eq!(before, after);
        assert_eq!(after.first().copied(), Some(root));
        assert!(after.contains(&c));
    }

    #[test]
    fn restructure_drops_dangling_and_unreachable() {
        let mut map = MindMap::default();
        let root = map.root;
        let a = map.add_child(root).unwrap();

        // Dangling child reference.
        map.get_mut(a).unwrap().children.push(NodeId(999));
        // Unreachable node.
        let orphan = Node {
            id: NodeId(500),
            parent: Some(NodeId(998)),
            children: vec![],
            pos: Pos2::new(10.0, 10.0),
            text: "orphan".to_owned(),
            details: String::new(),
        };
        map.nodes.insert(orphan.id, orphan);
        map.order.push(NodeId(500));

        assert!(map.restructure());
        assert!(!map.contains(NodeId(999)));
        assert!(!map.contains(NodeId(500)));
        assert!(map.contains(a));
    }

    #[test]
    fn restructure_rejects_missing_root() {
        let mut map = MindMap {
            root: NodeId(4242),
            ..Default::default()
        };
        assert!(!map.restructure());
    }

    /// T1.1 acceptance: a 5 000-node synthetic map loads and queries without
    /// quadratic slowdown.
    #[test]
    fn scale_5000_nodes() {
        let mut map = MindMap::default();
        let root = map.root;
        for i in 0..5000 {
            let id = map.add_child(root).expect("add under root");
            map.set_text(id, format!("node {i}"));
        }
        assert_eq!(map.len(), 5001);

        let start = Instant::now();
        for i in 0u64..5000 {
            let id = NodeId(i + 2);
            assert!(map.contains(id));
            assert!(map.get_mut(id).is_some());
        }
        // O(1) lookups make this milliseconds; the old O(n) scan would be
        // orders of magnitude slower through 25 M comparisons.
        assert!(
            start.elapsed().as_millis() < 2000,
            "lookups took {:?}",
            start.elapsed()
        );

        // Load path: serialize + parse + restructure the whole map.
        let started = Instant::now();
        let loaded = load_from_str(&save_to_string(&map).unwrap()).unwrap();
        assert_eq!(loaded.len(), map.len());
        assert!(
            started.elapsed().as_millis() < 2000,
            "load took {:?}",
            started.elapsed()
        );
    }
}
