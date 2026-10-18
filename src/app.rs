use std::path::PathBuf;

use eframe::egui::{self, Key, Layout, Ui};

use crate::canvas::{Canvas, EditorState, start_editing};
use crate::model::command::Command;
use crate::model::{self, BuiltInMap, MindMap};

const IDLE_HINT: &str = "Tab: child · Enter: sibling · F2/double-click: edit · Del: delete · \
                         Drag: pan · Ctrl+scroll: zoom · Ctrl+Z: undo · Ctrl+S: save";

const SHORTCUTS: &[(&str, &str)] = &[
    ("Add child of selection", "Tab"),
    ("Add sibling of selection", "Enter"),
    ("Edit selected node", "F2 or double-click"),
    ("Delete selected node", "Delete"),
    ("Undo / Redo", "Ctrl+Z / Ctrl+Shift+Z"),
    ("Deselect", "Escape / click background"),
    ("Save / Open / New", "Ctrl+S / Ctrl+O / Ctrl+N"),
    ("Zoom in / out", "Ctrl+= / Ctrl+-"),
    ("Fit all nodes in view", "Ctrl+0"),
    ("Pan", "Drag background, or scroll"),
    ("Zoom with mouse", "Ctrl+scroll"),
    ("Quit", "Ctrl+Q"),
];

#[derive(Clone)]
enum PendingOp {
    New,
    Open(PathBuf),
    Template(BuiltInMap),
    Quit,
}

pub struct MindMapApp {
    state: EditorState,
    canvas: Canvas,
    file_path: Option<PathBuf>,
    status: String,
    show_help: bool,
    pending: Option<PendingOp>,
    close_approved: bool,
    active_template: Option<BuiltInMap>,
    inspector_node: Option<model::NodeId>,
    details_draft: String,
}

impl MindMapApp {
    pub fn new(_cc: &eframe::CreationContext<'_>, initial_path: Option<String>) -> Self {
        let mut app = Self {
            state: EditorState::new(MindMap::default()),
            canvas: Canvas::default(),
            file_path: None,
            status: IDLE_HINT.to_owned(),
            show_help: false,
            pending: None,
            close_approved: false,
            active_template: Some(BuiltInMap::Blank),
            inspector_node: Some(MindMap::default().root),
            details_draft: String::new(),
        };
        if let Some(path) = initial_path {
            let path = PathBuf::from(path);
            if path.is_file() {
                app.do_open(path);
            } else {
                app.status = format!("No such file: {}", path.display());
            }
        }
        app
    }

    fn run_ui(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        self.handle_close_request(ctx);
        egui::Panel::top("top_navbar").show(ui, |ui| {
            self.menus(ui, ctx);
            ui.separator();
            self.toolbar(ui);
        });
        egui::Panel::bottom("status_bar").show(ui, |ui| self.status_bar(ui));
        self.shortcuts(ctx);
        self.help_window(ctx);
        self.confirm_dialog(ctx);
        self.state.input_enabled = self.pending.is_none();
        egui::Panel::right("node_inspector")
            .default_size(300.0)
            .resizable(true)
            .show(ui, |ui| self.node_inspector(ui));
        egui::CentralPanel::default().show(ui, |ui| {
            self.canvas.show(ui, &mut self.state);
        });
    }

    // ------------------------------------------------------------------ menus

    fn menus(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if menu_item(ui, "New", "Ctrl+N") {
                    ui.close();
                    self.request_new();
                }
                if menu_item(ui, "Open…", "Ctrl+O") {
                    ui.close();
                    self.request_open();
                }
                ui.separator();
                if menu_item(ui, "Save", "Ctrl+S") {
                    ui.close();
                    self.save();
                }
                if menu_item(ui, "Save as…", "") {
                    ui.close();
                    self.save_as();
                }
                if menu_item(ui, "Export image (SVG)…", "") {
                    ui.close();
                    self.export_image();
                }
                ui.separator();
                if menu_item(ui, "Quit", "Ctrl+Q") {
                    ui.close();
                    self.request_quit(ctx);
                }
            });
            ui.menu_button("Edit", |ui| {
                let can_undo = self.state.history.can_undo();
                if menu_item_enabled(ui, "Undo", "Ctrl+Z", can_undo) {
                    ui.close();
                    self.undo();
                }
                let can_redo = self.state.history.can_redo();
                if menu_item_enabled(ui, "Redo", "Ctrl+Shift+Z", can_redo) {
                    ui.close();
                    self.redo();
                }
                ui.separator();
                if menu_item(ui, "Delete selected", "Del") {
                    ui.close();
                    self.delete_selected();
                }
            });
            ui.menu_button("View", |ui| {
                if menu_item(ui, "Zoom in", "Ctrl+=") {
                    ui.close();
                    self.canvas.zoom_by(1.25);
                }
                if menu_item(ui, "Zoom out", "Ctrl+-") {
                    ui.close();
                    self.canvas.zoom_by(1.0 / 1.25);
                }
                if menu_item(ui, "Fit all nodes", "Ctrl+0") {
                    ui.close();
                    self.fit_view();
                }
                let mut branch_only = self.canvas.branch_only;
                if ui
                    .checkbox(&mut branch_only, "Only selected branch")
                    .changed()
                {
                    self.canvas.branch_only = branch_only;
                    self.fit_view();
                }
            });
            ui.menu_button("Templates", |ui| {
                for template in BuiltInMap::ALL {
                    if ui
                        .button(template.label())
                        .on_hover_text(template.description())
                        .clicked()
                    {
                        ui.close();
                        self.request_template(template);
                    }
                }
            });
            ui.menu_button("Help", |ui| {
                if menu_item(ui, "Keyboard shortcuts", "") {
                    ui.close();
                    self.show_help = true;
                }
            });
        });
    }

    fn template_switcher(&mut self, ui: &mut Ui) {
        ui.label("Template:");
        let mut chosen = self.active_template;
        egui::ComboBox::from_id_salt("built_in_template_switcher")
            .selected_text(
                self.active_template
                    .map(BuiltInMap::label)
                    .unwrap_or("Custom map"),
            )
            .show_ui(ui, |ui| {
                for template in BuiltInMap::ALL {
                    ui.selectable_value(&mut chosen, Some(template), template.label())
                        .on_hover_text(template.description());
                }
            });
        if chosen != self.active_template
            && let Some(template) = chosen
        {
            self.request_template(template);
        }
    }

    fn toolbar(&mut self, ui: &mut Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("Mindmap").strong());
            ui.separator();

            if ui
                .button("New")
                .on_hover_text("Create a blank mindmap (Ctrl+N)")
                .clicked()
            {
                self.request_new();
            }
            if ui
                .button("Open")
                .on_hover_text("Open a JSON mindmap (Ctrl+O)")
                .clicked()
            {
                self.request_open();
            }
            if ui
                .button("Save")
                .on_hover_text("Save the current mindmap (Ctrl+S)")
                .clicked()
            {
                self.save();
            }
            ui.separator();

            let can_undo = self.state.history.can_undo();
            if ui
                .add_enabled(can_undo, egui::Button::new("Undo"))
                .on_hover_text("Undo the last change (Ctrl+Z)")
                .clicked()
            {
                self.undo();
            }
            let can_redo = self.state.history.can_redo();
            if ui
                .add_enabled(can_redo, egui::Button::new("Redo"))
                .on_hover_text("Redo the last undone change (Ctrl+Shift+Z)")
                .clicked()
            {
                self.redo();
            }
            ui.separator();

            if ui
                .button("＋ Child")
                .on_hover_text("Add a child node (Tab)")
                .clicked()
            {
                self.add_child();
            }
            if ui
                .button("＋ Sibling")
                .on_hover_text("Add a sibling node (Enter)")
                .clicked()
            {
                self.add_sibling();
            }
            if ui
                .button("Edit")
                .on_hover_text("Edit the selected node (F2)")
                .clicked()
            {
                self.begin_edit();
            }
            if ui
                .button("Delete")
                .on_hover_text("Delete the selected subtree (Delete)")
                .clicked()
            {
                self.delete_selected();
            }
            ui.separator();

            if ui.button("−").on_hover_text("Zoom out (Ctrl+-)").clicked() {
                self.canvas.zoom_by(1.0 / 1.25);
            }
            if ui
                .button("Fit")
                .on_hover_text("Fit every node in view (Ctrl+0)")
                .clicked()
            {
                self.fit_view();
            }
            if ui.button("＋").on_hover_text("Zoom in (Ctrl+=)").clicked() {
                self.canvas.zoom_by(1.25);
            }
            ui.separator();
            let was_branch_only = self.canvas.branch_only;
            ui.add_enabled_ui(self.state.selected.is_some(), |ui| {
                ui.checkbox(&mut self.canvas.branch_only, "Branch only")
                    .on_hover_text("Show only the selected node and its descendants");
            });
            if was_branch_only != self.canvas.branch_only {
                self.fit_view();
            }
            ui.separator();
            self.template_switcher(ui);
        });
    }

    fn status_bar(&mut self, ui: &mut Ui) {
        let (leaves, depth) = map_metrics(&self.state.map);
        let root_title = self
            .state
            .map
            .get(self.state.map.root)
            .map(|node| node.text.as_str())
            .unwrap_or("Untitled map");
        let source = self
            .file_path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "Not saved".to_owned());
        let template = self
            .active_template
            .map(BuiltInMap::label)
            .unwrap_or("Custom map");
        let selection = self
            .state
            .selected
            .and_then(|id| self.state.map.get(id))
            .map(|node| node.text.as_str())
            .unwrap_or("None");

        ui.vertical(|ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new(root_title).strong());
                ui.separator();
                ui.label(format!("Template: {template}"));
                ui.separator();
                ui.label(format!("{} nodes", self.state.map.len()));
                ui.separator();
                ui.label(format!("{leaves} leaves"));
                ui.separator();
                ui.label(format!("Depth: {depth}"));
                ui.separator();
                ui.label(format!("Selected: {selection}"));
                ui.separator();
                ui.label(format!("Zoom: {:.0}%", self.canvas.zoom * 100.0));
                ui.separator();
                ui.label(
                    if self.canvas.branch_only && self.state.selected.is_some() {
                        "View: selected branch"
                    } else {
                        "View: full map"
                    },
                );
                ui.separator();
                ui.label(if self.state.dirty {
                    "Unsaved changes"
                } else {
                    "Saved"
                });
            });
            ui.horizontal(|ui| {
                ui.label(&self.status);
                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("Source: {source}"));
                });
            });
        });
    }

    fn node_inspector(&mut self, ui: &mut Ui) {
        let selected = self.state.selected;
        if selected != self.inspector_node {
            self.flush_details();
            self.inspector_node = selected;
            self.details_draft = selected
                .and_then(|id| self.state.map.get(id))
                .map(|node| node.details.clone())
                .unwrap_or_default();
        }

        ui.heading("Node details");
        ui.separator();
        let Some(id) = selected else {
            ui.add_space(8.0);
            ui.label(egui::RichText::new("No node selected").strong());
            ui.label("Click a branch or the central topic to inspect it.");
            ui.add_space(8.0);
            ui.label("The canvas is ready for a new selection.");
            return;
        };

        let Some(node) = self.state.map.get(id) else {
            ui.label("The selected node is no longer available.");
            return;
        };
        ui.label(egui::RichText::new(&node.text).heading());
        ui.small(format!("Node #{}", id.0));
        ui.add_space(12.0);
        ui.label("Long description / notes");
        ui.add(
            egui::TextEdit::multiline(&mut self.details_draft)
                .desired_rows(14)
                .desired_width(f32::INFINITY)
                .hint_text("Write context, links, decisions, or next steps…"),
        );
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new("Notes are saved with the mindmap JSON file.")
                .small()
                .weak(),
        );
        let has_details = !node.details.is_empty() || !self.details_draft.is_empty();
        ui.horizontal(|ui| {
            if ui.button("Apply notes").clicked() {
                self.flush_details();
                self.status = "Saved notes for selected node".to_owned();
            }
            if ui
                .add_enabled(has_details, egui::Button::new("Clear description"))
                .on_hover_text("Remove all description text from this node")
                .clicked()
            {
                self.clear_details();
            }
        });
    }

    fn help_window(&mut self, ctx: &egui::Context) {
        if !self.show_help {
            return;
        }
        let mut open = true;
        egui::Window::new("Keyboard shortcuts")
            .collapsible(false)
            .resizable(false)
            .default_width(360.0)
            .open(&mut open)
            .show(ctx, |ui| {
                egui::Grid::new("shortcut_grid")
                    .num_columns(2)
                    .spacing([28.0, 5.0])
                    .show(ui, |ui| {
                        for (action, keys) in SHORTCUTS {
                            ui.label(*action);
                            ui.monospace(*keys);
                            ui.end_row();
                        }
                    });
            });
        self.show_help = open;
    }

    // ------------------------------------------------------------- shortcuts

    fn shortcuts(&mut self, ctx: &egui::Context) {
        if self.pending.is_some() || self.state.editing.is_some() {
            return;
        }
        let ctrl = ctx.input(|i| i.modifiers.command);
        let shift = ctx.input(|i| i.modifiers.shift);
        let pressed = |key: Key| ctx.input(move |i| i.key_pressed(key));

        if ctrl {
            if pressed(Key::Z) {
                if shift {
                    self.redo();
                } else {
                    self.undo();
                }
            } else if pressed(Key::Y) {
                self.redo();
            } else if pressed(Key::S) {
                self.save();
            } else if pressed(Key::O) {
                self.request_open();
            } else if pressed(Key::N) {
                self.request_new();
            } else if pressed(Key::Q) {
                self.request_quit(ctx);
            } else if pressed(Key::Plus) || pressed(Key::Equals) {
                self.canvas.zoom_by(1.25);
            } else if pressed(Key::Minus) {
                self.canvas.zoom_by(1.0 / 1.25);
            } else if pressed(Key::Num0) {
                self.fit_view();
            }
            return;
        }

        if pressed(Key::Tab) {
            self.add_child();
        } else if pressed(Key::Enter) {
            self.add_sibling();
        } else if pressed(Key::F2) {
            self.begin_edit();
        } else if pressed(Key::Delete) || pressed(Key::Backspace) {
            self.delete_selected();
        } else if pressed(Key::Escape) {
            self.state.selected = None;
        }
    }

    // ------------------------------------------------------------- structure

    fn add_child(&mut self) {
        let target = self.state.selected.unwrap_or(self.state.map.root);
        let Some((id, cmd)) = self.state.map.add_child_cmd(target) else {
            return;
        };
        if self.state.run(cmd) {
            self.state.selected = Some(id);
            start_editing(&mut self.state.editing, &self.state.map, id);
            self.status = "New node — type a label, Enter confirms".to_owned();
        }
    }

    fn add_sibling(&mut self) {
        let Some(selected) = self.state.selected else {
            self.add_child();
            return;
        };
        let Some((id, cmd)) = self.state.map.add_sibling_cmd(selected) else {
            return;
        };
        if self.state.run(cmd) {
            self.state.selected = Some(id);
            start_editing(&mut self.state.editing, &self.state.map, id);
            self.status = "New node — type a label, Enter confirms".to_owned();
        }
    }

    fn begin_edit(&mut self) {
        if let Some(id) = self.state.selected {
            start_editing(&mut self.state.editing, &self.state.map, id);
        }
    }

    fn delete_selected(&mut self) {
        self.flush_details();
        let Some(id) = self.state.selected else {
            return;
        };
        if self.state.editing.as_ref().is_some_and(|e| e.id == id) {
            self.state.editing = None;
        }
        if self.state.run(Command::DeleteSubtree { id }) {
            self.state.selected = None;
            self.status = "Deleted node".to_owned();
        } else {
            self.status = "The central node can't be deleted".to_owned();
        }
    }

    fn undo(&mut self) {
        self.flush_details();
        match self.state.undo() {
            Some(what) => {
                self.validate_selection();
                self.status = format!("Undid {what}");
            }
            None => self.status = "Nothing to undo".to_owned(),
        }
    }

    fn redo(&mut self) {
        self.flush_details();
        match self.state.redo() {
            Some(what) => {
                self.validate_selection();
                self.status = format!("Redid {what}");
            }
            None => self.status = "Nothing to redo".to_owned(),
        }
    }

    /// Undo/redo can remove the node the selection points at.
    fn validate_selection(&mut self) {
        if let Some(sel) = self.state.selected
            && !self.state.map.contains(sel)
        {
            self.state.selected = None;
        }
    }

    fn fit_view(&mut self) {
        let bounds = if self.canvas.branch_only {
            self.state
                .selected
                .and_then(|id| self.state.map.subtree_bounds(id))
                .or_else(|| self.state.map.bounds())
        } else {
            self.state.map.bounds()
        };
        if let Some(bounds) = bounds {
            self.canvas.center_on(bounds);
        }
    }

    // ------------------------------------------------------------- files

    fn flush_details(&mut self) {
        let Some(id) = self.inspector_node else {
            return;
        };
        let Some(old) = self.state.map.get(id).map(|node| node.details.clone()) else {
            return;
        };
        if old != self.details_draft {
            self.state.run(Command::SetDetails {
                id,
                old,
                new: self.details_draft.clone(),
            });
        }
    }

    fn clear_details(&mut self) {
        self.flush_details();
        let Some(id) = self.inspector_node else {
            return;
        };
        let Some(old) = self.state.map.get(id).map(|node| node.details.clone()) else {
            return;
        };
        if !old.is_empty() {
            self.state.run(Command::SetDetails {
                id,
                old,
                new: String::new(),
            });
        }
        self.details_draft.clear();
        self.status = "Cleared description for selected node".to_owned();
    }

    /// Writes any in-progress text edit into the map (as a command, T1.3).
    fn flush_edit(&mut self) {
        self.flush_details();
        let Some(editing) = self.state.editing.take() else {
            return;
        };
        if let Some(old) = self.state.map.get(editing.id).map(|n| n.text.clone()) {
            self.state.run(Command::SetText {
                id: editing.id,
                old,
                new: editing.text,
            });
        }
    }

    fn request_new(&mut self) {
        self.flush_edit();
        if self.state.dirty {
            self.pending = Some(PendingOp::New);
        } else {
            self.do_new();
        }
    }

    fn do_new(&mut self) {
        self.state = EditorState::new(MindMap::default());
        self.inspector_node = Some(self.state.map.root);
        self.details_draft.clear();
        self.file_path = None;
        self.active_template = Some(BuiltInMap::Blank);
        self.canvas = Canvas::default();
        self.status = "New mind map".to_owned();
    }

    fn request_template(&mut self, template: BuiltInMap) {
        if self.active_template == Some(template) {
            return;
        }
        self.flush_edit();
        if self.state.dirty {
            self.pending = Some(PendingOp::Template(template));
        } else {
            self.apply_template(template);
        }
    }

    fn apply_template(&mut self, template: BuiltInMap) {
        self.state = EditorState::new(template.map());
        self.inspector_node = Some(self.state.map.root);
        self.details_draft.clear();
        self.file_path = None;
        self.active_template = Some(template);
        self.canvas = Canvas::default();
        self.fit_view();
        self.status = format!("Loaded {}", template.label());
    }

    fn request_open(&mut self) {
        self.flush_edit();
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Mindmap", &["json"])
            .pick_file()
        else {
            return;
        };
        if self.state.dirty {
            self.pending = Some(PendingOp::Open(path));
        } else {
            self.do_open(path);
        }
    }

    fn do_open(&mut self, path: PathBuf) {
        match model::load_file(&path) {
            Ok(map) => {
                self.state = EditorState::new(map);
                self.inspector_node = Some(self.state.map.root);
                self.details_draft.clear();
                self.file_path = Some(path.clone());
                self.active_template = None;
                self.fit_view();
                self.status = format!("Opened {}", path.display());
            }
            Err(err) => self.status = format!("Could not open file: {err}"),
        }
    }

    fn save(&mut self) {
        self.flush_edit();
        match self.file_path.clone() {
            Some(path) => self.do_save(path),
            None => self.save_as(),
        }
    }

    fn save_as(&mut self) {
        self.flush_edit();
        let suggested = self
            .file_path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "mindmap.json".to_owned());
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Mindmap", &["json"])
            .set_file_name(suggested)
            .save_file()
        else {
            return;
        };
        self.do_save(path);
    }

    fn export_image(&mut self) {
        self.flush_edit();
        let Some(path) = rfd::FileDialog::new()
            .add_filter("SVG image", &["svg"])
            .set_file_name("mindmap.svg")
            .save_file()
        else {
            return;
        };
        let branch_root = self
            .canvas
            .branch_only
            .then_some(self.state.selected)
            .flatten();
        match model::save_svg(&self.state.map, &path, branch_root) {
            Ok(()) => self.status = format!("Exported image to {}", path.display()),
            Err(err) => self.status = format!("Could not export image: {err}"),
        }
    }

    fn do_save(&mut self, path: PathBuf) {
        match model::save_file(&self.state.map, &path) {
            Ok(()) => {
                self.file_path = Some(path.clone());
                self.state.dirty = false;
                self.status = format!("Saved {}", path.display());
            }
            Err(err) => self.status = format!("Could not save file: {err}"),
        }
    }

    fn request_quit(&mut self, ctx: &egui::Context) {
        self.flush_edit();
        if self.state.dirty {
            self.pending = Some(PendingOp::Quit);
        } else {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    fn handle_close_request(&mut self, ctx: &egui::Context) {
        if self.close_approved
            || !ctx.input(|i| i.viewport().close_requested())
            || self.pending.is_some()
        {
            return;
        }
        self.flush_edit();
        if self.state.dirty {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.pending = Some(PendingOp::Quit);
        }
    }

    // ------------------------------------------------------------- dialog

    fn confirm_dialog(&mut self, ctx: &egui::Context) {
        let Some(pending) = self.pending.clone() else {
            return;
        };
        let mut choice: Option<u8> = None;
        egui::Modal::new(egui::Id::unique("unsaved_changes")).show(ctx, |ui| {
            ui.set_min_width(340.0);
            ui.heading("Unsaved changes");
            ui.label("Do you want to save your changes before continuing?");
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    choice = Some(1);
                }
                if ui.button("Don't save").clicked() {
                    choice = Some(2);
                }
                if ui.button("Cancel").clicked() {
                    choice = Some(3);
                }
            });
        });
        match choice {
            Some(1) => {
                self.save();
                if !self.state.dirty {
                    self.pending = None;
                    self.run_pending(pending, ctx);
                }
            }
            Some(2) => {
                self.pending = None;
                self.run_pending(pending, ctx);
            }
            Some(3) => {
                self.pending = None;
            }
            _ => {}
        }
    }

    fn run_pending(&mut self, op: PendingOp, ctx: &egui::Context) {
        match op {
            PendingOp::New => self.do_new(),
            PendingOp::Open(path) => self.do_open(path),
            PendingOp::Template(template) => self.apply_template(template),
            PendingOp::Quit => {
                // The user explicitly chose "Don't save". Mark the close as
                // approved so the next close-request frame does not prompt
                // again just because the in-memory document is still dirty.
                self.state.dirty = false;
                self.close_approved = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }
}

impl eframe::App for MindMapApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.run_ui(ui, &ctx);
    }
}

fn map_metrics(map: &MindMap) -> (usize, usize) {
    let leaves = map.iter().filter(|node| node.children.is_empty()).count();
    let mut deepest = 1;
    let mut stack = vec![(map.root, 1usize)];
    while let Some((id, level)) = stack.pop() {
        deepest = deepest.max(level);
        if let Some(node) = map.get(id) {
            stack.extend(node.children.iter().map(|child| (*child, level + 1)));
        }
    }
    (leaves, deepest)
}

fn menu_item(ui: &mut Ui, label: &str, shortcut: &str) -> bool {
    let mut button = egui::Button::new(label);
    if !shortcut.is_empty() {
        button = button.shortcut_text(shortcut);
    }
    ui.add(button).clicked()
}

fn menu_item_enabled(ui: &mut Ui, label: &str, shortcut: &str, enabled: bool) -> bool {
    let mut button = egui::Button::new(label);
    if !shortcut.is_empty() {
        button = button.shortcut_text(shortcut);
    }
    ui.add_enabled(enabled, button).clicked()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_app() -> MindMapApp {
        let map = MindMap::default();
        MindMapApp {
            state: EditorState::new(map.clone()),
            canvas: Canvas::default(),
            file_path: None,
            status: String::new(),
            show_help: false,
            pending: None,
            close_approved: false,
            active_template: Some(BuiltInMap::Blank),
            inspector_node: Some(map.root),
            details_draft: String::new(),
        }
    }

    #[test]
    fn edit_action_starts_editing_the_selected_node() {
        let mut app = test_app();
        let root = app.state.map.root;
        app.state.selected = Some(root);

        app.begin_edit();

        let editing = app.state.editing.expect("selected node should be editable");
        assert_eq!(editing.id, root);
        assert_eq!(editing.text, "Central Topic");
        assert!(editing.fresh);
    }

    #[test]
    fn clear_description_removes_persisted_node_details() {
        let mut app = test_app();
        let root = app.state.map.root;
        app.state
            .map
            .set_details(root, "Details to remove".to_owned());
        app.inspector_node = Some(root);
        app.details_draft = "Details to remove".to_owned();

        app.clear_details();

        assert!(app.state.map.get(root).unwrap().details.is_empty());
        assert!(app.details_draft.is_empty());
        assert!(app.state.dirty);
    }

    #[test]
    fn dont_save_new_discards_the_dirty_document() {
        let mut app = test_app();
        app.add_child();
        assert!(app.state.dirty);

        app.run_pending(PendingOp::New, &egui::Context::default());

        assert_eq!(app.state.map.len(), 1);
        assert!(!app.state.dirty);
        assert!(app.state.editing.is_none());
    }

    #[test]
    fn dont_save_quit_approves_close_without_reprompting() {
        let mut app = test_app();
        app.state.dirty = true;

        app.run_pending(PendingOp::Quit, &egui::Context::default());

        assert!(!app.state.dirty);
        assert!(app.close_approved);
    }
}
