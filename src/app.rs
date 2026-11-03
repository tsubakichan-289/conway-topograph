use crate::render::{self, Disk};
use eframe::egui::{self, Color32};
use std::collections::BTreeSet;
use topograph_core::{
    algebra::{Form, Primitive, parse_integer},
    editor::Editor,
    geometry::{Complex, Frame, Isometry},
    scene::{Limits, Scene, SceneCache},
    topology::Triangle,
};

pub struct App {
    pub editor: Editor,
    pub frame: Frame,
    pub view: Isometry,
    pub scene: Scene,
    coefficient_inputs: [String; 3],
    fixed_inputs: Vec<String>,
    selected: Option<Primitive>,
    selected_triangle: Option<Triangle>,
    label_input: String,
    pub labels: bool,
    pub rivers: bool,
    pub wells: bool,
    pub lakes: bool,
    pub zoom: f32,
    pub density: f64,
    pub depth: usize,
    pub palette: render::Palette,
    pub color_mode: render::ColorMode,
    pub graph_mode: render::GraphMode,
    pub value_scale: u32,
    message: String,
    previous_pointer: Option<egui::Pos2>,
    pub export_path: String,
    river_previous: Option<Triangle>,
    river_current: Option<Triangle>,
    cache: SceneCache,
    export_request: Option<bool>,
    last_radius: f64,
    last_disk: Option<Disk>,
}
impl Default for App {
    fn default() -> Self {
        let mut app = Self {
            editor: Editor::default(),
            frame: Frame::new(Triangle::root()),
            view: Isometry::default(),
            scene: Scene::default(),
            coefficient_inputs: Default::default(),
            fixed_inputs: Vec::new(),
            selected: None,
            selected_triangle: None,
            label_input: String::new(),
            labels: true,
            rivers: true,
            wells: true,
            lakes: true,
            zoom: 1.0,
            density: 6.0,
            depth: 32,
            palette: render::Palette::default(),
            color_mode: render::ColorMode::default(),
            graph_mode: render::GraphMode::default(),
            value_scale: render::default_value_scale(),
            message: String::new(),
            previous_pointer: None,
            export_path: "topograph".to_owned(),
            river_previous: None,
            river_current: None,
            cache: SceneCache::default(),
            export_request: None,
            last_radius: 400.0,
            last_disk: None,
        };
        app.sync_inputs();
        app
    }
}
impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        let mut app = Self::default();
        if let Some(json) = cc.storage.and_then(|s| s.get_string("conway-exact-v1")) {
            match crate::storage::Saved::from_json(&json).and_then(|saved| app.restore(saved)) {
                Ok(()) => {}
                Err(e) => app.message = format!("Could not restore settings: {e}"),
            }
        }
        app
    }
    fn restore(
        &mut self,
        saved: crate::storage::Saved,
    ) -> Result<(), crate::storage::StorageError> {
        let (editor, frame) = saved.validate()?;
        self.editor = editor;
        self.frame = frame;
        self.view = saved.view;
        self.labels = saved.labels;
        self.rivers = saved.rivers;
        self.wells = saved.wells;
        self.lakes = saved.lakes;
        self.zoom = saved.zoom;
        self.depth = saved.depth;
        self.density = saved.density;
        self.palette = saved.palette;
        self.color_mode = saved.color_mode;
        self.graph_mode = saved.graph_mode;
        self.value_scale = saved.value_scale;
        self.selected = None;
        self.selected_triangle = None;
        self.river_current = None;
        self.river_previous = None;
        self.sync_inputs();
        Ok(())
    }
    fn sync_inputs(&mut self) {
        let f = &self.editor.form;
        self.coefficient_inputs = [f.a.to_string(), f.b.to_string(), f.c.to_string()];
        self.fixed_inputs = self
            .editor
            .fixed
            .iter()
            .map(|p| p.value.to_string())
            .collect();
        if let Some(v) = &self.selected {
            self.label_input = f.evaluate(v).to_string();
        }
    }
    fn reset(&mut self) {
        self.frame = Frame::new(Triangle::root());
        self.view = Isometry::default();
        self.zoom = 1.0;
        self.river_previous = None;
        self.river_current = None;
    }
    fn commit_coefficients(&mut self) {
        let result = (|| {
            let a = parse_integer(&self.coefficient_inputs[0])?;
            let b = parse_integer(&self.coefficient_inputs[1])?;
            let c = parse_integer(&self.coefficient_inputs[2])?;
            Ok::<_, topograph_core::algebra::MathError>(Form { a, b, c })
        })();
        match result {
            Ok(f) => {
                self.editor.set_form(f);
                self.sync_inputs();
                self.message = "Form updated.".into();
            }
            Err(e) => self.message = e.to_string(),
        }
    }
    fn commit_labels(&mut self) {
        let result = self
            .fixed_inputs
            .iter()
            .map(|s| parse_integer(s))
            .collect::<Result<Vec<_>, _>>()
            .map_err(topograph_core::editor::EditError::from)
            .and_then(|v| self.editor.apply_labels(&v));
        match result {
            Ok(()) => {
                self.sync_inputs();
                self.message = "Form reconstructed exactly.".into();
            }
            Err(e) => self.message = e.to_string(),
        }
    }
    fn controls(&mut self, ui: &mut egui::Ui) {
        ui.heading("Q(x, y)");
        for (i, label) in ["a", "b", "c"].iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(*label);
                ui.text_edit_singleline(&mut self.coefficient_inputs[i]);
            });
        }
        if ui.button("Apply coefficients").clicked() {
            self.commit_coefficients();
        }
        ui.label(format!("D = {}", self.editor.form.discriminant()));
        ui.separator();
        ui.collapsing("Fixed faces", |ui| {
            let mut remove = None;
            let mut replace = None;
            for i in 0..self.editor.fixed.len() {
                ui.horizontal(|ui| {
                    ui.label(self.editor.fixed[i].face.to_string());
                    if ui.small_button("Remove").clicked() {
                        remove = Some(i);
                    }
                    if ui
                        .add_enabled(
                            self.selected.is_some(),
                            egui::Button::new("Replace").small(),
                        )
                        .clicked()
                    {
                        replace = Some(i);
                    }
                });
                ui.text_edit_singleline(&mut self.fixed_inputs[i]);
            }
            if let Some(i) = remove {
                self.editor.fixed.remove(i);
                self.sync_inputs();
            }
            if let Some(i) = replace {
                let result = self.editor.replace(i, self.selected.clone().unwrap());
                self.message = result
                    .err()
                    .map_or("Fixed face replaced.".into(), |e| e.to_string());
                self.sync_inputs();
            }
            if ui.button("Apply three labels").clicked() {
                self.commit_labels();
            }
        });
        if ui
            .button("Reset view")
            .on_hover_text(
                "Drag to pan · wheel to scale · click a face to edit · double-click to center",
            )
            .clicked()
        {
            self.reset();
        }
        ui.separator();
        ui.collapsing("Display", |ui| {
            ui.label("Graph");
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.graph_mode, render::GraphMode::Topograph, "Topograph");
                ui.selectable_value(&mut self.graph_mode, render::GraphMode::Dual, "Dual");
                ui.selectable_value(&mut self.graph_mode, render::GraphMode::Overlay, "Overlay");
            }).response.on_hover_text("Dual: Farey triangles whose sides bisect topograph edges at right angles in the hyperbolic disk. Overlay: show both graphs.");
            ui.separator();
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.color_mode, render::ColorMode::Normal, "Normal");
                ui.selectable_value(&mut self.color_mode, render::ColorMode::ValueColors, "Value colors");
            });
            if self.color_mode == render::ColorMode::ValueColors {
                ui.add(egui::Slider::new(&mut self.value_scale, 1..=1_000_000)
                    .logarithmic(true).text("Color scale")
                    .custom_formatter(|n,_|format!("{n:.0}")))
                    .on_hover_text("Larger scale keeps larger values bright. Positive: red; negative: blue; zero: lake. Magnitude equal to the scale gives half brightness.");
            }
            ui.checkbox(&mut self.labels, "Face labels");
            ui.checkbox(&mut self.rivers, "River");
            ui.checkbox(&mut self.wells, "Well");
            ui.checkbox(&mut self.lakes, "Lake");
            ui.add(egui::Slider::new(&mut self.density, 2.0..=18.0).text("Density"));
            ui.add(egui::Slider::new(&mut self.depth, 8..=64).text("Depth"));
        });
        ui.collapsing("Colors", |ui| {
            for (label, color) in [
                ("Disk background", &mut self.palette.background),
                ("Canvas background", &mut self.palette.canvas),
                ("Edges", &mut self.palette.edge),
                ("Labels", &mut self.palette.text),
                ("Disk boundary", &mut self.palette.boundary),
            ] {
                ui.horizontal(|ui| {
                    ui.color_edit_button_srgb(color);
                    ui.label(label);
                });
            }
        });
        ui.collapsing("Explore",|ui| {
        if self.editor.form.square_discriminant() {
            ui.label("Square D: rational zero directions; lake structure.");
        }
        if ui
            .button("Find well / river / lake (exact descent)")
            .clicked()
        {
            let start = self
                .selected_triangle
                .clone()
                .unwrap_or(self.frame.triangle.clone());
            match topograph_core::features::seek_landmark(&self.editor.form, start, 4096) {
                Ok(t) => {
                    self.frame = Frame::new(t.clone());
                    self.view = Isometry::default();
                    self.selected = Some(t.faces()[0].clone());
                    self.label_input = self.editor.form.evaluate(&t.faces()[0]).to_string();
                    self.selected_triangle = Some(t);
                    self.river_current = None;
                    self.river_previous = None;
                }
                Err(e) => self.message = e.into(),
            }
        }
        ui.horizontal(|ui| {
            if ui.button("River next").clicked() {
                self.step_river(false);
            }
            if ui.button("Reverse").clicked() {
                self.step_river(true);
            }
        });
        ui.collapsing("Well evidence: all three edge directions",|ui| {
            let mut count=0;
            for v in &self.scene.vertices {
                if let Some(w)=topograph_core::features::well(&self.editor.form,&v.triangle) {
                    count+=1;
                    ui.label(format!("{:?} · {}",w.kind,if w.negative {"ascent of -Q"} else {"ascent of Q"}));
                    for i in 0..3 {ui.label(format!("{} : Q={} · outward ΔQ={}",v.triangle.faces()[i],w.values[i],w.deltas[i]));}
                }
            }
            if count==0 {ui.label("No definite source in the visible region.");}
            ui.label("Single: 3 strict ascents. Double: 2 ascents + 1 flat edge. For negative definite forms, apply the rule to -Q.");
        });
        });
        ui.collapsing("Export / files", |ui| {
            ui.label("File");
            ui.text_edit_singleline(&mut self.export_path);
            ui.horizontal(|ui| {
                if ui.button("Export SVG").clicked() {
                    self.export_request = Some(false);
                }
                if ui.button("Export PNG").clicked() {
                    self.export_request = Some(true);
                }
            });
            ui.horizontal(|ui| {
                if ui.button("Save JSON").clicked() {
                    let path = std::path::Path::new(&self.export_path).with_extension("json");
                    let result = crate::storage::Saved::capture(self)
                        .to_json()
                        .and_then(|json| {
                            std::fs::write(&path, json)?;
                            Ok(())
                        });
                    self.message = result
                        .err()
                        .map_or(format!("Saved {}", path.display()), |e| e.to_string());
                }
                if ui.button("Load JSON").clicked() {
                    let path = std::path::Path::new(&self.export_path).with_extension("json");
                    let result = std::fs::read_to_string(&path)
                        .map_err(crate::storage::StorageError::from)
                        .and_then(|s| crate::storage::Saved::from_json(&s))
                        .and_then(|s| self.restore(s));
                    self.message = result
                        .err()
                        .map_or(format!("Loaded {}", path.display()), |e| e.to_string());
                }
            });
        });
        if self.scene.truncated {
            ui.colored_label(
                Color32::YELLOW,
                "Vertex budget reached; lower density or move view.",
            );
        }
        if !self.message.is_empty() {
            ui.separator();
            ui.label(&self.message);
        }
    }
    fn selection(&mut self, ui: &mut egui::Ui) {
        let Some(face) = self.selected.clone() else {
            ui.label("Click a face region to inspect its primitive vector and exact label.");
            return;
        };
        ui.horizontal_wrapped(|ui| {
            ui.label(format!(
                "Face {face}    Q = {}",
                self.editor.form.evaluate(&face)
            ));
            ui.text_edit_singleline(&mut self.label_input);
            if ui.button("Fix this face").clicked() {
                let result = self.editor.pin(face.clone());
                self.message = result.err().map_or("Face fixed.".into(), |e| e.to_string());
                self.sync_inputs();
            }
            if ui.button("Apply selected label").clicked() {
                let result = parse_integer(&self.label_input)
                    .map_err(topograph_core::editor::EditError::from)
                    .and_then(|v| self.editor.edit_face(&face, v));
                self.message = result
                    .err()
                    .map_or("Label updated through three fixed faces.".into(), |e| {
                        e.to_string()
                    });
                if self.editor.form.evaluate(&face).to_string() == self.label_input {
                    self.sync_inputs();
                }
            }
        });
        ui.collapsing("Face details", |ui| {
            let mut adjacent = BTreeSet::new();
            let mut vertices = Vec::new();
            let mut edges = 0;
            for v in &self.scene.vertices {
                if v.triangle.faces().contains(&face) {
                    vertices.push(v.triangle.clone());
                    for other in v.triangle.faces() {
                        if other != &face {
                            adjacent.insert(other.clone());
                        }
                    }
                }
            }
            for e in &self.scene.edges {
                if e.faces.contains(&face) {
                    edges += 1;
                }
            }
            ui.label(format!(
                "Visible adjacent faces: {}",
                adjacent
                    .iter()
                    .take(16)
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" · ")
            ));
            ui.label(format!(
                "Visible incident vertices: {}    boundary edges: {} (finite view)",
                vertices.len(),
                edges
            ));
            ui.collapsing("Incident vertices and edges (visible portion)", |ui| {
                for t in vertices.iter().take(32) {
                    ui.label(format!(
                        "Vertex: {}",
                        t.faces()
                            .iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>()
                            .join(" / ")
                    ));
                }
                for e in self
                    .scene
                    .edges
                    .iter()
                    .filter(|e| e.faces.contains(&face))
                    .take(32)
                {
                    ui.label(format!("Edge faces: {} / {}", e.faces[0], e.faces[1]));
                }
                if vertices.len() > 32 || edges > 32 {
                    ui.label("List limited to 32 entries; pan to inspect the remaining portion.");
                }
            });
            if let Some(t) = &self.selected_triangle {
                ui.label(format!(
                    "Selected vertex: {}",
                    t.faces()
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(" / ")
                ));
                if let Some(w) = topograph_core::features::well(&self.editor.form, t) {
                    ui.colored_label(
                        render::rgb(render::GOLD),
                        format!(
                            "{:?} well · {} · Q values {:?} · outward ΔQ {:?}",
                            w.kind,
                            if w.negative {
                                "ascent of -Q / descent of Q"
                            } else {
                                "ascent of Q"
                            },
                            w.values,
                            w.deltas
                        ),
                    );
                } else {
                    ui.label(format!(
                        "Outward ΔQ: {:?} (not a definite source)",
                        topograph_core::features::changes(&self.editor.form, t)
                    ));
                }
            }
        });
    }
    fn step_river(&mut self, reverse: bool) {
        let form = &self.editor.form;
        let current = self.river_current.clone().or_else(|| {
            self.scene
                .edges
                .iter()
                .filter(|e| topograph_core::features::river(form, &e.faces))
                .min_by(|a, b| {
                    self.frame
                        .center(&a.from)
                        .norm()
                        .total_cmp(&self.frame.center(&b.from).norm())
                })
                .map(|e| e.from.clone())
        });
        let Some(current) = current else {
            self.message =
                "No nonzero sign separator in this view. Use exact descent to find a bank or lake."
                    .into();
            return;
        };
        let next = if reverse && self.river_previous.is_some() {
            self.river_previous.clone()
        } else {
            topograph_core::features::river_neighbors(form, &current)
                .into_iter()
                .find(|t| Some(t) != self.river_previous.as_ref())
        };
        if let Some(next) = next {
            self.river_previous = Some(current);
            self.river_current = Some(next.clone());
            self.selected_triangle = Some(next.clone());
            self.frame = Frame::new(next);
            self.view = Isometry::default();
            self.message = "Moved by one exact river edge.".into();
        } else {
            self.message = "Separator ends at a lake (square discriminant).".into();
        }
    }
    pub fn render_options(&self) -> render::Options {
        render::Options {
            labels: self.labels,
            rivers: self.rivers,
            wells: self.wells,
            lakes: self.lakes,
            selected: self.selected.clone(),
            palette: self.palette,
            color_mode: self.color_mode,
            graph_mode: self.graph_mode,
            value_scale: self.value_scale,
        }
    }
    fn rebase(&mut self) {
        if let Some(v) = self.scene.nearest_vertex(Complex::ZERO)
            && v.triangle != self.frame.triangle
            && v.center.norm() < 0.45
        {
            let new = Frame::new(v.triangle.clone());
            self.view = self.view.compose(self.frame.transition(&new));
            self.frame = new;
        }
    }
    fn canvas(&mut self, ui: &mut egui::Ui) {
        let (response, painter) =
            ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());
        let rect = response.rect;
        painter.rect_filled(rect, 0.0, render::rgb(self.palette.canvas));
        let disk = Disk {
            center: rect.center(),
            radius: rect.width().min(rect.height()) * 0.47 * self.zoom,
        };
        self.last_disk = Some(disk);
        let pointer = response.interact_pointer_pos();
        if response.dragged()
            && let (Some(previous), Some(current)) = (self.previous_pointer, pointer)
            && disk.point(previous).norm() < 0.98
            && disk.point(current).norm() < 0.98
        {
            self.view = self.view.drag(disk.point(previous), disk.point(current));
        }
        self.previous_pointer = pointer;
        if response.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                self.zoom = (self.zoom * (scroll * 0.002).exp()).clamp(0.35, 1.0);
            }
        }
        if let Some(scene) = self.cache.update(
            &self.frame,
            self.view,
            Limits {
                max_depth: self.depth,
                radius: disk.radius as f64,
                min_pixels: self.density,
                ..Default::default()
            },
        ) {
            self.scene = scene;
        }
        self.last_radius = disk.radius as f64;
        if (response.clicked() || response.double_clicked())
            && let Some(point) = pointer.map(|p| disk.point(p))
        {
            if let Some(patch) = self.scene.face_at(point) {
                self.selected = Some(patch.face.clone());
                self.selected_triangle = Some(patch.triangle.clone());
                self.label_input = self.editor.form.evaluate(&patch.face).to_string();
            }
            if response.double_clicked() && point.norm() < 0.98 {
                self.view = self.view.center(point);
            }
        }
        render::paint(
            &painter,
            &self.scene,
            &self.editor.form,
            disk,
            &self.render_options(),
        );
        if let Some(png) = self.export_request.take() {
            let path = std::path::Path::new(&self.export_path).with_extension(if png {
                "png"
            } else {
                "svg"
            });
            let result = crate::export::svg(
                &self.scene,
                &self.editor.form,
                &self.render_options(),
                self.last_radius,
                1600,
            )
            .and_then(|svg| {
                if png {
                    crate::export::save_png(&path, &svg)
                } else {
                    crate::export::save_svg(&path, &svg)
                }
            });
            self.message = result
                .err()
                .map_or(format!("Exported {}", path.display()), |e| e.to_string());
        }
        self.rebase();
    }
    pub fn show(&mut self, ui: &mut egui::Ui) {
        egui::Panel::left("controls")
            .default_size(245.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| self.controls(ui));
            });
        if self.selected.is_some() {
            egui::Panel::bottom("selection")
                .default_size(72.0)
                .show(ui, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| self.selection(ui));
                });
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::central_panel(ui.style()).fill(render::rgb(self.palette.canvas)))
            .show(ui, |ui| self.canvas(ui));
    }
}
impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        if let Ok(json) = crate::storage::Saved::capture(self).to_json() {
            storage.set_string("conway-exact-v1", json);
        }
    }
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        self.show(ui);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn frame(app: &mut App, ctx: &egui::Context, events: Vec<egui::Event>) -> egui::FullOutput {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1360.0, 980.0),
                )),
                events,
                ..Default::default()
            },
            |ui| app.show(ui),
        );
        output.textures_delta.clear();
        output
    }
    fn button(pos: egui::Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        }
    }
    #[test]
    fn restored_graph_modes_render_in_egui() {
        let ctx = egui::Context::default();
        let mut app = App::default();
        for mode in [
            render::GraphMode::Dual,
            render::GraphMode::Overlay,
            render::GraphMode::Topograph,
        ] {
            let mut saved = crate::storage::Saved::capture(&app);
            saved.graph_mode = mode;
            app.restore(saved).unwrap();
            assert_eq!(app.render_options().graph_mode, mode);
            let output = frame(&mut app, &ctx, vec![]);
            assert!(!output.shapes.is_empty());
            assert!(!app.scene.dual_edges.is_empty());
        }
    }
    #[test]
    fn headless_egui_selects_faces_and_pans_hyperbolically() {
        let ctx = egui::Context::default();
        let mut app = App::default();
        for _ in 0..3 {
            let output = frame(&mut app, &ctx, vec![]);
            assert!(!output.shapes.is_empty());
        }
        let disk = app.last_disk.unwrap();
        let patch = app
            .scene
            .patches
            .iter()
            .find(|p| p.clearance > 20.0)
            .unwrap();
        let expected = patch.face.clone();
        let pos = disk.screen(patch.label);
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(pos), button(pos, true)],
        );
        frame(&mut app, &ctx, vec![button(pos, false)]);
        assert_eq!(app.selected, Some(expected));
        // Selecting a face reveals the compact inspector and changes the canvas layout.
        for _ in 0..3 {
            frame(&mut app, &ctx, vec![]);
        }
        let disk = app.last_disk.unwrap();
        let a = disk.screen(Complex::new(-0.15, 0.1));
        let b = disk.screen(Complex::new(0.05, 0.2));
        let initial = app.view.apply(Complex::ZERO);
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(a), button(a, true)],
        );
        frame(&mut app, &ctx, vec![egui::Event::PointerMoved(b)]);
        frame(&mut app, &ctx, vec![button(b, false)]);
        assert!((app.view.apply(Complex::ZERO) - initial).norm() > 0.01);
        assert!(app.view.valid());
        assert!(app.scene.vertices.iter().all(|v| v.center.norm() < 1.0));
        app.reset();
        assert_eq!(app.frame.triangle, Triangle::root());
        assert_eq!(app.view, Isometry::default());
    }
}
