use std::collections::HashMap;

use egui::containers::{DragPanButtons, Scene};
use egui::epaint::CubicBezierShape;
use egui::text::{CCursor, CCursorRange};
use egui::text_edit::TextEditState;
use egui::{
    Align2, Color32, CursorIcon, FontId, Key, Pos2, Rangef, Rect, Sense, Stroke, StrokeKind,
    TextStyle, Ui, Vec2,
};

use crate::model::{MindMap, NodeId};

const NODE_TEXT_SIZE: f32 = 15.0;
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

pub struct Canvas {
    scene: Scene,
    pub scene_rect: Rect,
    pub zoom: f32,
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

    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ui: &mut Ui,
        map: &mut MindMap,
        selected: &mut Option<NodeId>,
        editing: &mut Option<Editing>,
        dirty: &mut bool,
        input_enabled: bool,
    ) {
        let outer_rect = ui.available_rect_before_wrap();
        let Canvas {
            scene,
            scene_rect,
            zoom,
        } = self;

        let inner = scene.show(ui, scene_rect, |ui| {
            let ctx = ui.ctx().clone();
            let visuals = ui.visuals().clone();
            let root_font = ui.style().text_styles[&TextStyle::Heading].clone();
            let node_font = FontId::proportional(NODE_TEXT_SIZE);
            let text_color = visuals.text_color();

            // --- measure every node so edges and hit-rects line up -------------
            let mut sizes: HashMap<NodeId, Vec2> = HashMap::with_capacity(map.nodes.len());
            for node in &map.nodes {
                let font = if node.id == map.root {
                    root_font.clone()
                } else {
                    node_font.clone()
                };
                let text = match editing {
                    Some(e) if e.id == node.id => e.text.clone(),
                    _ => node.text.clone(),
                };
                let galley = ui.painter().layout_no_wrap(text, font, text_color);
                let mut size = galley.size() + NODE_PAD * 2.0;
                size.x = size.x.max(MIN_NODE_WIDTH);
                size.y = size.y.max(MIN_NODE_HEIGHT);
                sizes.insert(node.id, size);
            }

            // --- edges ----------------------------------------------------------
            let edge_color = visuals.weak_text_color();
            for node in &map.nodes {
                let Some(parent_id) = node.parent else {
                    continue;
                };
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

            for node in &map.nodes {
                let id = node.id;
                let font = if id == map.root {
                    root_font.clone()
                } else {
                    node_font.clone()
                };
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
                    if ed.fresh {
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
                    } else if enter || resp.lost_focus() || click_outside {
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
                ui.painter().text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    &node.text,
                    font,
                    text_color,
                );
            }

            // --- apply deferred interaction results ------------------------------
            for (id, delta) in drag_updates {
                if let Some(node) = map.get_mut(id) {
                    node.pos += delta;
                    *dirty = true;
                }
            }
            if let Some((id, text)) = finish {
                *editing = None;
                if let Some(text) = text
                    && map.set_text(id, text)
                {
                    *dirty = true;
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
