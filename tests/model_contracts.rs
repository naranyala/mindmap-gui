use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::model::command::Command;
use crate::model::{BuiltInMap, MindMap, NodeId, load_from_str, save_to_string, save_svg};

static NEXT_TEMP_FILE: AtomicU64 = AtomicU64::new(0);

fn temp_file(extension: &str) -> PathBuf {
    let id = NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "mindmap-gui-test-{}-{id}.{extension}",
        std::process::id()
    ))
}

#[test]
fn save_load_roundtrip_preserves_tree_order_and_node_content() {
    let mut map = MindMap::default();
    let root = map.root;
    let first = map.add_child(root).unwrap();
    let second = map.add_child(root).unwrap();
    let inserted = map.add_sibling(first).unwrap();
    map.set_text(first, "Research".to_owned());
    map.set_details(first, "Sources and open questions".to_owned());

    let expected_order: Vec<NodeId> = map.iter().map(|node| node.id).collect();
    let loaded = load_from_str(&save_to_string(&map).unwrap()).unwrap();

    assert_eq!(loaded.iter().map(|node| node.id).collect::<Vec<_>>(), expected_order);
    assert_eq!(loaded.get(root).unwrap().children, vec![first, inserted, second]);
    assert_eq!(loaded.get(first).unwrap().text, "Research");
    assert_eq!(loaded.get(first).unwrap().details, "Sources and open questions");
    assert_eq!(loaded.get(first).unwrap().parent, Some(root));
}

#[test]
fn loader_rejects_future_file_versions_with_a_clear_error() {
    let json = save_to_string(&MindMap::default())
        .unwrap()
        .replace("\"version\": 1", "\"version\": 99");

    let error = load_from_str(&json).unwrap_err();
    assert!(error.contains("version 99"), "unexpected error: {error}");
    assert!(error.contains("supported version 1"), "unexpected error: {error}");
}

#[test]
fn loader_rejects_a_document_without_a_valid_root() {
    let json = r#"{"app":"mindmap-gui","version":1,"map":{"root":999,"nodes":[],"next_id":1}}"#;
    assert_eq!(load_from_str(json).unwrap_err(), "file has no valid root node");
}

#[test]
fn delete_command_restores_a_subtree_in_its_original_sibling_slot() {
    let mut map = MindMap::default();
    let root = map.root;
    let before = map.add_child(root).unwrap();
    let removed = map.add_child(root).unwrap();
    let grandchild = map.add_child(removed).unwrap();
    let after = map.add_child(root).unwrap();

    let restore = Command::DeleteSubtree { id: removed }
        .run(&mut map)
        .expect("non-root subtree should be deletable");
    assert_eq!(map.get(root).unwrap().children, vec![before, after]);
    assert!(!map.contains(removed));
    assert!(!map.contains(grandchild));

    let delete_again = restore.run(&mut map).expect("deleted subtree should restore");
    assert_eq!(map.get(root).unwrap().children, vec![before, removed, after]);
    assert_eq!(map.get(removed).unwrap().children, vec![grandchild]);
    assert_eq!(map.get(grandchild).unwrap().parent, Some(removed));
    assert!(delete_again.run(&mut map).is_some());
    assert_eq!(map.get(root).unwrap().children, vec![before, after]);
}

#[test]
fn every_builtin_template_has_consistent_parent_child_links() {
    for template in BuiltInMap::ALL {
        let map = template.map();
        let mut visited = std::collections::HashSet::new();
        let mut stack = vec![map.root];

        while let Some(parent_id) = stack.pop() {
            assert!(visited.insert(parent_id), "cycle in {}", template.label());
            let parent = map.get(parent_id).expect("child reference must resolve");
            let mut direct_children = std::collections::HashSet::new();
            for &child_id in &parent.children {
                assert!(direct_children.insert(child_id), "duplicate child in {}", template.label());
                let child = map.get(child_id).expect("child reference must resolve");
                assert_eq!(child.parent, Some(parent_id), "broken parent in {}", template.label());
                stack.push(child_id);
            }
        }

        assert_eq!(visited.len(), map.len(), "unreachable node in {}", template.label());
        assert_eq!(map.get(map.root).unwrap().parent, None);
    }
}

#[test]
fn svg_export_escapes_node_text_and_limits_output_to_requested_branch() {
    let mut map = MindMap::default();
    let root = map.root;
    let branch = map.add_child(root).unwrap();
    let leaf = map.add_child(branch).unwrap();
    let sibling = map.add_child(root).unwrap();
    map.set_text(branch, "A <B> & \"quotes\" 'apostrophe'".to_owned());
    map.set_text(leaf, "Visible leaf".to_owned());
    map.set_text(sibling, "Excluded sibling".to_owned());

    let path = temp_file("svg");
    let result = save_svg(&map, &path, Some(branch));
    assert!(result.is_ok(), "SVG export failed: {result:?}");
    let svg = fs::read_to_string(&path).unwrap();
    let _ = fs::remove_file(&path);

    assert!(svg.contains("A &lt;B&gt; &amp; &quot;quotes&quot; &apos;apostrophe&apos;"));
    assert!(svg.contains("Visible leaf"));
    assert!(!svg.contains("Excluded sibling"));
    assert!(!svg.contains("Central Topic"));
}

#[test]
fn subtree_bounds_include_descendants_but_exclude_siblings() {
    let mut map = MindMap::default();
    let root = map.root;
    let branch = map.add_child(root).unwrap();
    let child = map.add_child(branch).unwrap();
    let sibling = map.add_child(root).unwrap();
    map.get_mut(branch).unwrap().pos = egui::pos2(100.0, 50.0);
    map.get_mut(child).unwrap().pos = egui::pos2(300.0, -20.0);
    map.get_mut(sibling).unwrap().pos = egui::pos2(10_000.0, 10_000.0);

    let bounds = map.subtree_bounds(branch).unwrap();
    assert!(bounds.contains(map.get(branch).unwrap().pos));
    assert!(bounds.contains(map.get(child).unwrap().pos));
    assert!(!bounds.contains(map.get(sibling).unwrap().pos));
}

#[test]
fn malformed_json_is_reported_instead_of_panicking() {
    let error = load_from_str("{ definitely not json }").unwrap_err();
    assert!(!error.is_empty());
}

#[test]
fn missing_node_lookups_are_safe_and_do_not_mutate_the_map() {
    let mut map = MindMap::default();
    let initial_len = map.len();
    let missing = NodeId(u64::MAX);

    assert!(map.get(missing).is_none());
    assert!(map.get_mut(missing).is_none());
    assert!(!map.contains(missing));
    assert_eq!(map.len(), initial_len);
}
