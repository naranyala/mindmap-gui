use std::path::PathBuf;

use eframe::egui::{self, Key, Layout, Ui};

use crate::canvas::{Canvas, Editing, start_editing};
use crate::model::{self, MindMap, NodeId};

const IDLE_HINT: &str = "Tab: child · Enter: sibling · F2/double-click: edit · Del: delete · \
                         Drag: pan · Ctrl+scroll: zoom · Ctrl+S: save";

const SHORTCUTS: &[(&str, &str)] = &[
    ("Add child of selection", "Tab"),
    ("Add sibling of selection", "Enter"),
    ("Edit selected node", "F2 or double-click"),
    ("Delete selected node", "Delete"),
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
    Quit,
}

pub struct MindMapApp {
    map: MindMap,
    canvas: Canvas,
    selected: Option<NodeId>,
    editing: Option<Editing>,
    dirty: bool,
    file_path: Option<PathBuf>,
    status: String,
    show_help: bool,
    pending: Option<PendingOp>,
}

impl MindMapApp {
    pub fn new(_cc: &eframe::CreationContext<'_>, initial_path: Option<String>) -> Self {
        let map = MindMap::default();
        let selected = Some(map.root);
        let mut app = Self {
            map,
            canvas: Canvas::default(),
            selected,
            editing: None,
            dirty: false,
            file_path: None,
            status: IDLE_HINT.to_owned(),
            show_help: false,
            pending: None,
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
        egui::Panel::top("main_menu").show(ui, |ui| self.menus(ui, ctx));
        egui::Panel::bottom("status_bar").show(ui, |ui| self.status_bar(ui));
        self.shortcuts(ctx);
        self.help_window(ctx);
        self.confirm_dialog(ctx);
        egui::CentralPanel::default().show(ui, |ui| {
            self.canvas.show(
                ui,
                &mut self.map,
                &mut self.selected,
                &mut self.editing,
                &mut self.dirty,
                self.pending.is_none(),
            );
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
                ui.separator();
                if menu_item(ui, "Quit", "Ctrl+Q") {
                    ui.close();
                    self.request_quit(ctx);
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
            });
            ui.menu_button("Help", |ui| {
                if menu_item(ui, "Keyboard shortcuts", "") {
                    ui.close();
                    self.show_help = true;
                }
            });
        });
    }

    fn status_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(&self.status);
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(format!("{:.0}%", self.canvas.zoom * 100.0));
                ui.separator();
                ui.label(format!("{} nodes", self.map.nodes.len()));
                ui.separator();
                let name = self
                    .file_path
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "Untitled".to_owned());
                ui.label(format!("{name}{}", if self.dirty { " •" } else { "" }));
            });
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
        if self.pending.is_some() || self.editing.is_some() {
            return;
        }
        let ctrl = ctx.input(|i| i.modifiers.command);
        let pressed = |key: Key| ctx.input(move |i| i.key_pressed(key));

        if ctrl {
            if pressed(Key::S) {
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
            self.selected = None;
        }
    }

    // ------------------------------------------------------------- structure

    fn add_child(&mut self) {
        let target = self.selected.unwrap_or(self.map.root);
        if let Some(id) = self.map.add_child(target) {
            self.dirty = true;
            self.selected = Some(id);
            start_editing(&mut self.editing, &self.map, id);
            self.status = "New node — type a label, Enter confirms".to_owned();
        }
    }

    fn add_sibling(&mut self) {
        let Some(selected) = self.selected else {
            self.add_child();
            return;
        };
        if let Some(id) = self.map.add_sibling(selected) {
            self.dirty = true;
            self.selected = Some(id);
            start_editing(&mut self.editing, &self.map, id);
            self.status = "New node — type a label, Enter confirms".to_owned();
        }
    }

    fn begin_edit(&mut self) {
        if let Some(id) = self.selected {
            start_editing(&mut self.editing, &self.map, id);
        }
    }

    fn delete_selected(&mut self) {
        let Some(id) = self.selected else { return };
        if self.editing.as_ref().is_some_and(|e| e.id == id) {
            self.editing = None;
        }
        if self.map.delete_subtree(id) {
            self.dirty = true;
            self.selected = None;
            self.status = "Deleted node".to_owned();
        } else {
            self.status = "The central node can't be deleted".to_owned();
        }
    }

    fn fit_view(&mut self) {
        if let Some(bounds) = self.map.bounds() {
            self.canvas.center_on(bounds);
        }
    }

    // ------------------------------------------------------------- files

    /// Writes any in-progress text edit into the map.
    fn flush_edit(&mut self) {
        let Some(editing) = self.editing.take() else {
            return;
        };
        if self.map.set_text(editing.id, editing.text) {
            self.dirty = true;
        }
    }

    fn request_new(&mut self) {
        self.flush_edit();
        if self.dirty {
            self.pending = Some(PendingOp::New);
        } else {
            self.do_new();
        }
    }

    fn do_new(&mut self) {
        self.map = MindMap::default();
        self.selected = Some(self.map.root);
        self.editing = None;
        self.file_path = None;
        self.dirty = false;
        self.canvas = Canvas::default();
        self.status = "New mind map".to_owned();
    }

    fn request_open(&mut self) {
        self.flush_edit();
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Mindmap", &["json"])
            .pick_file()
        else {
            return;
        };
        if self.dirty {
            self.pending = Some(PendingOp::Open(path));
        } else {
            self.do_open(path);
        }
    }

    fn do_open(&mut self, path: PathBuf) {
        match model::load_file(&path) {
            Ok(map) => {
                self.map = map;
                self.selected = Some(self.map.root);
                self.editing = None;
                self.file_path = Some(path.clone());
                self.dirty = false;
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

    fn do_save(&mut self, path: PathBuf) {
        match model::save_file(&self.map, &path) {
            Ok(()) => {
                self.file_path = Some(path.clone());
                self.dirty = false;
                self.status = format!("Saved {}", path.display());
            }
            Err(err) => self.status = format!("Could not save file: {err}"),
        }
    }

    fn request_quit(&mut self, ctx: &egui::Context) {
        self.flush_edit();
        if self.dirty {
            self.pending = Some(PendingOp::Quit);
        } else {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    fn handle_close_request(&mut self, ctx: &egui::Context) {
        if !ctx.input(|i| i.viewport().close_requested()) || self.pending.is_some() {
            return;
        }
        self.flush_edit();
        if self.dirty {
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
                if !self.dirty {
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
            PendingOp::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
        }
    }
}

impl eframe::App for MindMapApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.run_ui(ui, &ctx);
    }
}

fn menu_item(ui: &mut Ui, label: &str, shortcut: &str) -> bool {
    let mut button = egui::Button::new(label);
    if !shortcut.is_empty() {
        button = button.shortcut_text(shortcut);
    }
    ui.add(button).clicked()
}
