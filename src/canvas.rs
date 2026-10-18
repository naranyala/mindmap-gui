use std::collections::HashMap;

use egui::containers::{DragPanButtons, Scene};
use egui::epaint::CubicBezierShape;
use egui::text::{CCursor, CCursorRange};
use egui::text_edit::TextEditState;
use egui::{
    Color32, CursorIcon, FontId, Key, Pos2, Rangef, Rect, Sense, Stroke, StrokeKind, TextStyle, Ui,
    Vec2,
};

use crate::model::command::Command;
use crate::model::history::History;
use crate::model::{MindMap, NodeId};

const NODE_PAD: Vec2 = Vec2::new(17.0, 10.0);
const MIN_NODE_WIDTH: f32 = 90.0;
const MIN_NODE_HEIGHT: f32 = 38.0;
const NODE_ROUNDING: f32 = 9.0;
const EDGE_WIDTH: f32 = 2.0;

/// State of an in-progress node text edit.
pub struct Editing {
    pub id: NodeId,
    pub text: String,
    pub fresh: bool,
}

pub fn start_editing(editing: &mut Option<Editing>, map: &MindMap, id: NodeId) {
    let Some(node) = map.get(id) else {
        return;
    };
    *editing = Some(Editing {
        id,
        text: node.text.clone(),
        fresh: true,
    });
}

/// Everything the canvas and the editor shell act on: the document, the
/// selection, the active inline edit, the dirty flag, and the undo history.
///
/// All mutations go through [`EditorState::run`], so every change is stored in
/// the history and undoable (T1.3).
///
/// Note: the pyramid's five-field bundle (`map, selected, editing, dirty,
/// canvas`) grows `history` and `input_enabled` here. Keeping them in the same
/// struct is what lets [`Canvas::show(&mut self, ui, &mut EditorState)`] stay
/// at three parameters (T1.2 acceptance) while canvas mutations still reach
/// the history.
pub struct EditorState {
    pub map: MindMap,
    pub selected: Option<NodeId>,
    pub editing: Option<Editing>,
    pub dirty: bool,
    pub history: History,
    /// False while a modal (e.g. the unsaved-changes dialog) blocks input.
    pub input_enabled: bool,
}

impl EditorState {
    pub fn new(map: MindMap) -> Self {
        Self {
            selected: Some(map.root),
            map,
            editing: None,
            dirty: false,
            history: History::default(),
            input_enabled: true,
        }
    }

    /// Applies a command through the history. Returns true if the map changed.
    pub fn run(&mut self, cmd: Command) -> bool {
        if self.history.run(cmd, &mut self.map) {
            self.dirty = true;
            true
        } else {
            false
        }
    }

    /// Undoes the newest step; returns its action label, if any.
    pub fn undo(&mut self) -> Option<&'static str> {
        let what = self.history.undo(&mut self.map)?;
        self.dirty = true;
        Some(what)
    }

    /// Redoes the most recently undone step; returns its action label, if any.
    pub fn redo(&mut self) -> Option<&'static str> {
        let what = self.history.redo(&mut self.map)?;
        self.dirty = true;
        Some(what)
    }
}

pub struct Canvas {
    scene: Scene,
    pub scene_rect: Rect,
    pub zoom: f32,
    pub branch_only: bool,
}

impl Default for Canvas {
    fn default() -> Self {
        Self {
            scene: Scene::new()
                .zoom_range(Rangef::new(0.1, 4.0))
                .max_inner_size(Vec2::splat(1_000_000.0))
                .drag_pan_buttons(DragPanButtons::PRIMARY | DragPanButtons::MIDDLE),
            scene_rect: Rect::from_center_size(Pos2::ZERO, Vec2::new(1400.0, 900.0)),
            zoom: 1.0,
            branch_only: false,
        }
    }
}

impl Canvas {
    /// Zoom around the center of the current view. `factor > 1` zooms in.
    pub fn zoom_by(&mut self, factor: f32) {
        let center = self.scene_rect.center();
        let size = (self.scene_rect.size() / factor).max(Vec2::splat(20.0));
        self.scene_rect = Rect::from_center_size(center, size);
    }

    /// Make the given scene-space rect fill the view.
    pub fn center_on(&mut self, rect: Rect) {
        if rect.size().x > 1.0 && rect.size().y > 1.0 {
            self.scene_rect = rect;
        }
    }

    pub fn show(&mut self, ui: &mut Ui, state: &mut EditorState) {
        let outer_rect = ui.available_rect_before_wrap();
        let Canvas {
            scene,
            scene_rect,
            zoom,
            ..
        } = self;
        let EditorState {
            map,
            selected,
            editing,
            dirty,
            history,
            input_enabled,
        } = state;
        let input_enabled = *input_enabled;
        let branch_only = self.branch_only;

        let inner = scene.show(ui, scene_rect, |ui| {
            let ctx = ui.ctx().clone();
            let visuals = ui.visuals().clone();
            let title_font = ui.style().text_styles[&TextStyle::Heading].clone();
            let text_color = visuals.text_color();
            let title_color = visuals.strong_text_color();

            // --- measure every node so edges and hit-rects line up -------------
            let visible_ids: std::collections::HashSet<NodeId> = if branch_only {
                selected
                    .map(|id| descendant_ids(map, id))
                    .unwrap_or_default()
            } else {
                map.iter().map(|node| node.id).collect()
            };
            let mut sizes: HashMap<NodeId, Vec2> = HashMap::with_capacity(visible_ids.len());
            for node in map.iter() {
                if !visible_ids.contains(&node.id) {
                    continue;
                }
                let font = title_font.clone();
                let text = match editing {
                    Some(e) if e.id == node.id => e.text.clone(),
                    _ => node.text.clone(),
                };
                let title_galley = ui.painter().layout_no_wrap(text, font, title_color);
                let details_font = FontId::proportional(12.0);
                let details_galley = ui.painter().layout(
                    node.details.clone(),
                    details_font,
                    text_color.gamma_multiply(0.75),
                    240.0,
                );
                let mut size = Vec2::new(
                    title_galley.size().x.max(details_galley.size().x),
                    title_galley.size().y + details_galley.size().y,
                ) + NODE_PAD * 2.0
                    + Vec2::new(0.0, 5.0);
                size.x = size.x.max(MIN_NODE_WIDTH);
                size.y = size.y.max(MIN_NODE_HEIGHT);
                sizes.insert(node.id, size);
            }

            // --- edges ----------------------------------------------------------
            let edge_color = visuals.weak_text_color();
            for node in map.iter() {
                if !visible_ids.contains(&node.id) {
                    continue;
                }
                let Some(parent_id) = node.parent else {
                    continue;
                };
                if !visible_ids.contains(&parent_id) {
                    continue;
                }
                let (Some(&psize), Some(&csize)) = (sizes.get(&parent_id), sizes.get(&node.id))
                else {
                    continue;
                };
                let (Some(parent), Some(child)) = (map.get(parent_id), map.get(node.id)) else {
                    continue;
                };
                let p0 = edge_anchor(parent.pos, psize, child.pos);
                let p3 = edge_anchor(child.pos, csize, parent.pos);
                let mid = Vec2::new((p3.x - p0.x) * 0.5, 0.0);
                ui.painter().add(CubicBezierShape::from_points_stroke(
                    [p0, p0 + mid, p3 - mid, p3],
                    false,
                    Color32::TRANSPARENT,
                    Stroke::new(EDGE_WIDTH, edge_color),
                ));
            }

            // --- nodes ----------------------------------------------------------
            let mut finish: Option<(NodeId, Option<String>)> = None;
            let mut drag_updates: Vec<(NodeId, Vec2)> = Vec::new();
            let mut clicked: Option<NodeId> = None;
            let mut double_clicked: Option<NodeId> = None;
            let mut dragged: Option<NodeId> = None;

            for node in map.iter() {
                let id = node.id;
                if !visible_ids.contains(&id) {
                    continue;
                }
                let font = title_font.clone();
                let size = sizes[&id];
                let rect = Rect::from_center_size(node.pos, size);
                let is_selected = *selected == Some(id);

                if editing.as_ref().is_some_and(|e| e.id == id) {
                    let edit_id = ui.make_persistent_id(("mindmap_node_edit", id.0));
                    let Some(ed) = editing.as_mut() else {
                        continue;
                    };
                    let resp = ui.put(
                        rect,
                        egui::TextEdit::singleline(&mut ed.text)
                            .font(font)
                            .id_salt(("mindmap_node_edit", id.0))
                            .desired_width(f32::INFINITY),
                    );
                    let just_started = ed.fresh;
                    if just_started {
                        resp.request_focus();
                        let mut state = TextEditState::load(&ctx, edit_id).unwrap_or_default();
                        let end = ed.text.chars().count();
                        state.cursor.set_char_range(Some(CCursorRange::two(
                            CCursor::new(0),
                            CCursor::new(end),
                        )));
                        state.store(&ctx, edit_id);
                        ed.fresh = false;
                    }
                    let escape = input_enabled && ctx.input(|i| i.key_pressed(Key::Escape));
                    let enter = input_enabled && ctx.input(|i| i.key_pressed(Key::Enter));
                    let click_outside = input_enabled
                        && ctx.input(|i| i.pointer.any_click())
                        && !resp.contains_pointer();
                    if escape {
                        finish = Some((id, None));
                    } else if should_finish_edit(
                        just_started,
                        enter,
                        resp.lost_focus(),
                        click_outside,
                    ) {
                        finish = Some((id, Some(ed.text.clone())));
                    }
                    continue;
                }

                let response = ui.interact(
                    rect,
                    ui.make_persistent_id(("mindmap_node", id.0)),
                    Sense::click_and_drag(),
                );
                let hovered = input_enabled && response.hovered();

                if input_enabled {
                    if response.dragged() {
                        let delta = response.drag_delta();
                        if delta != Vec2::ZERO {
                            drag_updates.push((id, delta));
                        }
                        dragged = Some(id);
                        ctx.set_cursor_icon(CursorIcon::Grabbing);
                    } else if response.hovered() {
                        ctx.set_cursor_icon(CursorIcon::Grab);
                    }
                    if response.double_clicked() {
                        double_clicked = Some(id);
                    } else if response.clicked() {
                        clicked = Some(id);
                    }
                }

                let is_root = id == map.root;
                let fill = if is_selected {
                    visuals.selection.bg_fill
                } else if is_root {
                    visuals.widgets.active.bg_fill
                } else if hovered {
                    visuals.widgets.hovered.bg_fill
                } else {
                    visuals.widgets.inactive.bg_fill
                };
                let stroke = if is_selected {
                    Stroke::new(2.0, visuals.selection.stroke.color)
                } else if is_root {
                    Stroke::new(1.5, visuals.selection.stroke.color.gamma_multiply(0.6))
                } else {
                    Stroke::new(1.0, visuals.widgets.noninteractive.bg_stroke.color)
                };

                ui.painter().rect_filled(rect, NODE_ROUNDING, fill);
                ui.painter()
                    .rect_stroke(rect, NODE_ROUNDING, stroke, StrokeKind::Outside);
                if is_selected {
                    ui.painter().rect_stroke(
                        rect.expand(3.0),
                        NODE_ROUNDING + 2.0,
                        Stroke::new(3.0, visuals.selection.stroke.color.gamma_multiply(0.45)),
                        StrokeKind::Outside,
                    );
                }
                let title_galley =
                    ui.painter()
                        .layout_no_wrap(node.text.clone(), font, title_color);
                let details_galley = ui.painter().layout(
                    node.details.clone(),
                    FontId::proportional(12.0),
                    text_color.gamma_multiply(0.75),
                    240.0,
                );
                let content_height = title_galley.size().y + details_galley.size().y + 5.0;
                let title_height = title_galley.size().y;
                let content_top = rect.center().y - content_height / 2.0;
                let title_pos =
                    Pos2::new(rect.center().x - title_galley.size().x / 2.0, content_top);
                // egui's default font set has no separate bold face. A tiny
                // offset duplicate gives titles a reliable bold treatment in
                // both light and dark themes while keeping the heading size.
                ui.painter()
                    .galley(title_pos, title_galley.clone(), title_color);
                ui.painter()
                    .galley(title_pos + Vec2::new(0.55, 0.0), title_galley, title_color);
                ui.painter().galley(
                    Pos2::new(
                        rect.center().x - details_galley.size().x / 2.0,
                        content_top + title_height + 5.0,
                    ),
                    details_galley,
                    text_color.gamma_multiply(0.75),
                );
            }

            // --- apply deferred interaction results ------------------------------
            for (id, delta) in drag_updates {
                let Some(old_pos) = map.get(id).map(|n| n.pos) else {
                    continue;
                };
                let applied = history.run(
                    Command::MoveNode {
                        id,
                        old_pos,
                        new_pos: old_pos + delta,
                    },
                    map,
                );
                *dirty |= applied;
            }
            if let Some((id, text)) = finish {
                *editing = None;
                if let Some(text) = text
                    && let Some(old) = map.get(id).map(|n| n.text.clone())
                {
                    let applied = history.run(Command::SetText { id, old, new: text }, map);
                    *dirty |= applied;
                }
            }
            if let Some(id) = double_clicked.or(clicked).or(dragged) {
                *selected = Some(id);
            }
            if let Some(id) = double_clicked {
                start_editing(editing, map, id);
            }
        });

        if input_enabled && inner.response.clicked() {
            *selected = None;
        }

        let size = scene_rect.size();
        if size.x > 1.0 && size.y > 1.0 {
            *zoom = (outer_rect.width() / size.x).min(outer_rect.height() / size.y);
        }
    }
}

fn descendant_ids(map: &MindMap, root: NodeId) -> std::collections::HashSet<NodeId> {
    let mut ids = std::collections::HashSet::new();
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        if ids.insert(id)
            && let Some(node) = map.get(id)
        {
            stack.extend(node.children.iter().copied());
        }
    }
    ids
}

/// Where an edge should leave/touch a node rect, heading towards `target`.
fn edge_anchor(center: Pos2, size: Vec2, target: Pos2) -> Pos2 {
    let dir = target - center;
    if dir == Vec2::ZERO {
        return center;
    }
    let half = size / 2.0;
    let mut s = f32::MAX;
    if dir.x.abs() > 1e-4 {
        s = s.min(half.x / dir.x.abs());
    }
    if dir.y.abs() > 1e-4 {
        s = s.min(half.y / dir.y.abs());
    }
    if !s.is_finite() {
        return center;
    }
    center + dir * s
}

fn should_finish_edit(
    just_started: bool,
    enter: bool,
    lost_focus: bool,
    click_outside: bool,
) -> bool {
    !just_started && (enter || lost_focus || click_outside)
}

#[cfg(test)]
mod tests {
    use super::should_finish_edit;

    #[test]
    fn newly_started_editor_ignores_the_click_that_started_it() {
        assert!(!should_finish_edit(true, false, true, true));
    }

    #[test]
    fn existing_editor_finishes_on_outside_click() {
        assert!(should_finish_edit(false, false, false, true));
    }
}
