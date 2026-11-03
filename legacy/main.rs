mod background;
mod export;
mod graph;

use eframe::egui::{self, Align2, Color32, FontId, Pos2, Sense, Stroke, Vec2};
use graph::Graph;

const NODE_RADIUS: f32 = 18.0;
const ACCENT: Color32 = Color32::from_rgb(80, 210, 180);
const ANGLE_COLOR: Color32 = Color32::from_rgb(230, 70, 70);
const STORAGE_KEY: &str = "topograph";

fn segment_distance(point: Pos2, a: Pos2, b: Pos2) -> f32 {
    let direction = b - a;
    let length_sq = direction.length_sq();
    if length_sq == 0.0 {
        return point.distance(a);
    }
    let t = ((point - a).dot(direction) / length_sq).clamp(0.0, 1.0);
    point.distance(a + direction * t)
}

#[derive(Default)]
struct Topograph {
    graph: Graph,
    selected: Option<usize>,
    path_start: Option<usize>,
    history: Vec<Graph>,
    message: String,
    pan: Vec2,
    zoom: f32,
    dragging: Option<usize>,
    panning: bool,
    monochrome: bool,
    export_open: bool,
    export_path: String,
    export_status: String,
    background: background::Background,
}

impl Topograph {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        let graph: Graph = cc
            .storage
            .and_then(|s| eframe::get_value(s, STORAGE_KEY))
            .filter(Graph::is_valid)
            .unwrap_or_default();
        Self {
            graph,
            zoom: 1.0,
            export_path: "topograph.typ".into(),
            background: background::Background::restore(cc),
            ..Default::default()
        }
    }

    fn remember(&mut self) {
        if self.history.len() == 100 {
            self.history.remove(0);
        }
        self.history.push(self.graph.clone());
    }

    fn export_window(&mut self, ctx: &egui::Context) {
        egui::Window::new("Export CeTZ")
            .open(&mut self.export_open)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("Export black edges and dots, including double lines.");
                ui.label("Save as (.typ):");
                ui.add(egui::TextEdit::singleline(&mut self.export_path).desired_width(420.0));
                let path = std::path::PathBuf::from(self.export_path.trim());
                let path = if path.extension().is_none() {
                    path.with_extension("typ")
                } else {
                    path
                };
                if let Ok(cwd) = std::env::current_dir() {
                    ui.small(format!("Relative paths start at: {}", cwd.display()));
                }
                ui.horizontal(|ui| {
                    let label = if path.exists() {
                        "Save (overwrite existing file)"
                    } else {
                        "Save"
                    };
                    if ui
                        .add_enabled(
                            !self.export_path.trim().is_empty(),
                            egui::Button::new(label),
                        )
                        .clicked()
                    {
                        match std::fs::write(&path, export::to_cetz(&self.graph)) {
                            Ok(()) => {
                                let saved = path.canonicalize().unwrap_or(path);
                                self.export_status = format!("Saved: {}", saved.display());
                            }
                            Err(error) => self.export_status = format!("Could not save: {error}"),
                        }
                    }
                    if ui.button("Copy CeTZ").clicked() {
                        ctx.copy_text(export::to_cetz(&self.graph));
                        self.export_status = "Copied CeTZ source to clipboard.".into();
                    }
                });
                ui.label(&self.export_status);
            });
    }

    fn cancel(&mut self) {
        self.selected = None;
        self.path_start = None;
        self.message.clear();
    }

    fn undo(&mut self) {
        self.dragging = None;
        self.panning = false;
        if let Some(graph) = self.history.pop() {
            self.graph = graph;
        }
        self.cancel();
    }

    fn toggle_edge(&mut self, edge: usize) {
        self.remember();
        if !self.graph.double_edges.remove(&edge) {
            self.graph.double_edges.insert(edge);
        }
        self.message.clear();
    }

    fn remove_node(&mut self, node: usize) {
        if node >= self.graph.nodes.len() {
            return;
        }
        self.remember();
        self.graph.remove_node(node);
        self.dragging = None;
        self.panning = false;
        self.cancel();
    }

    fn click(&mut self, hit: Option<usize>, position: [f32; 2], extend: bool) {
        self.path_start = None;
        self.message.clear();
        match (self.selected, hit) {
            (None, None) => {
                if !self.graph.nodes.is_empty() {
                    return;
                }
                self.remember();
                let node = self.graph.add_node(position);
                self.selected = extend.then_some(node);
            }
            (None, Some(node)) => {
                if self.graph.degree(node) == 3 {
                    self.message = "This node already has 3 edges.".into();
                } else {
                    self.selected = Some(node);
                }
            }
            (Some(start), Some(end)) => {
                let previous = self.graph.clone();
                match self.graph.connect(start, end) {
                    Ok(()) => {
                        // Record only successful changes.
                        if self.history.len() == 100 {
                            self.history.remove(0);
                        }
                        self.history.push(previous);
                        self.selected = (extend && self.graph.degree(end) < 3).then_some(end);
                    }
                    Err(message) => self.message = message.into(),
                }
            }
            (Some(start), None) => {
                self.remember();
                let end = self.graph.add_node(position);
                // A selected node always has capacity; no other edit can add an edge
                // without clearing the selection.
                self.graph
                    .connect(start, end)
                    .expect("selected node has capacity");
                self.selected = extend.then_some(end);
            }
        }
    }

    fn ctrl_click_node(&mut self, node: usize) {
        self.selected = None;
        self.message.clear();
        let Some(start) = self.path_start else {
            self.path_start = Some(node);
            return;
        };
        if start == node {
            self.message = "Choose a different node for the path.".into();
            return;
        }
        let Some(edges) = self.graph.path_edges(start, node) else {
            self.message = "No path connects these nodes. Ctrl+click another endpoint.".into();
            return;
        };
        self.remember();
        let make_double = !edges
            .iter()
            .all(|edge| self.graph.double_edges.contains(edge));
        for edge in edges {
            if make_double {
                self.graph.double_edges.insert(edge);
            } else {
                self.graph.double_edges.remove(&edge);
            }
        }
        self.path_start = None;
    }

    fn canvas(&mut self, ui: &mut egui::Ui) {
        let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
        let rect = response.rect;
        painter.rect_filled(
            rect,
            0.0,
            if self.monochrome {
                Color32::WHITE
            } else {
                Color32::from_rgb(19, 24, 33)
            },
        );
        let hit_radius = if self.monochrome {
            12.0
        } else {
            NODE_RADIUS + 6.0
        };

        let pointer = response
            .interact_pointer_pos()
            .or_else(|| response.hover_pos());
        if response.hovered() {
            // egui maps Shift+wheel to horizontal scrolling. Use both axes so
            // zoom also continues through the smoothed tail after Shift release.
            let scroll = ui.input(|i| i.smooth_scroll_delta.x + i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                let old_zoom = self.zoom;
                self.zoom = (self.zoom * (scroll * 0.002).exp()).clamp(0.25, 30.0);
                if let Some(pos) = pointer {
                    let offset = pos - rect.min;
                    self.pan = offset - (offset - self.pan) * (self.zoom / old_zoom);
                }
            }
        }
        if response.dragged_by(egui::PointerButton::Middle) {
            self.pan += ui.input(|i| i.pointer.delta());
        }

        let origin = rect.min + self.pan;
        let screen = |p: [f32; 2]| origin + Vec2::from(p) * self.zoom;
        let hit = pointer.and_then(|pos| {
            self.graph
                .nodes
                .iter()
                .enumerate()
                .rev()
                .find(|(_, p)| screen(**p).distance(pos) <= hit_radius)
                .map(|(i, _)| i)
        });
        // Nodes take priority near endpoints. Pick the nearest segment at crossings.
        let edge_hit = pointer.filter(|_| hit.is_none()).and_then(|pos| {
            self.graph
                .edges
                .iter()
                .enumerate()
                .map(|(index, &[a, b])| {
                    (
                        index,
                        segment_distance(
                            pos,
                            screen(self.graph.nodes[a]),
                            screen(self.graph.nodes[b]),
                        ),
                    )
                })
                .filter(|&(_, distance)| distance <= 8.0)
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(index, _)| index)
        });

        if response.drag_started_by(egui::PointerButton::Primary) {
            // Use the press position so fast drags still pick up the node.
            self.dragging = ui.input(|i| i.pointer.press_origin()).and_then(|pos| {
                self.graph
                    .nodes
                    .iter()
                    .enumerate()
                    .rev()
                    .find(|(_, p)| screen(**p).distance(pos) <= hit_radius)
                    .map(|(i, _)| i)
            });
            self.panning = self.dragging.is_none();
            if self.dragging.is_some() {
                self.remember();
            }
        }
        if response.dragged_by(egui::PointerButton::Primary) {
            let delta = ui.input(|i| i.pointer.delta());
            if let Some(node) = self.dragging {
                self.graph.nodes[node][0] += delta.x / self.zoom;
                self.graph.nodes[node][1] += delta.y / self.zoom;
            } else if self.panning {
                self.pan += delta;
            }
        }
        if response.drag_stopped() {
            self.dragging = None;
            self.panning = false;
        }
        if response.secondary_clicked() {
            if let Some(node) = hit {
                self.remove_node(node);
            } else {
                self.cancel();
            }
        } else if response.clicked_by(egui::PointerButton::Primary)
            && let Some(pos) = pointer
        {
            if ui.input(|i| i.modifiers.ctrl) {
                if let Some(node) = hit {
                    self.ctrl_click_node(node);
                }
            } else if let Some(edge) = edge_hit {
                self.path_start = None;
                self.toggle_edge(edge);
            } else {
                let local = (pos - origin) / self.zoom;
                self.click(hit, [local.x, local.y], ui.input(|i| i.modifiers.shift));
            }
        }

        // Panning above must be reflected in this frame's drawing.
        let origin = rect.min + self.pan;
        if !self.monochrome {
            self.background.paint(&painter, origin, self.zoom);
        }
        let screen = |p: [f32; 2]| origin + Vec2::from(p) * self.zoom;
        let sharp_edges = self.graph.sharp_angle_edges();
        for (index, &[a, b]) in self.graph.edges.iter().enumerate() {
            let a = screen(self.graph.nodes[a]);
            let b = screen(self.graph.nodes[b]);
            let stroke = Stroke::new(
                2.5,
                if sharp_edges.contains(&index) {
                    ANGLE_COLOR
                } else if self.monochrome {
                    Color32::BLACK
                } else {
                    Color32::from_rgb(146, 162, 183)
                },
            );
            if self.graph.double_edges.contains(&index) {
                let direction = (b - a).normalized();
                let offset = Vec2::new(-direction.y, direction.x) * 3.0;
                painter.line_segment([a + offset, b + offset], stroke);
                painter.line_segment([a - offset, b - offset], stroke);
            } else {
                painter.line_segment([a, b], stroke);
            }
        }
        if let (Some(start), Some(pos)) = (self.selected, pointer) {
            painter.line_segment(
                [screen(self.graph.nodes[start]), pos],
                Stroke::new(
                    1.5,
                    if self.monochrome {
                        Color32::BLACK
                    } else {
                        ACCENT
                    },
                ),
            );
        }
        for (node, &position) in self.graph.nodes.iter().enumerate() {
            let pos = screen(position);
            if self.monochrome {
                painter.circle_filled(pos, 4.0, Color32::BLACK);
                if self.path_start == Some(node) {
                    painter.circle_stroke(pos, 8.0, Stroke::new(1.0, Color32::BLACK));
                }
                continue;
            }
            let selected = self.selected == Some(node) || self.path_start == Some(node);
            let color = if selected {
                ACCENT
            } else if self.graph.degree(node) == 3 {
                Color32::from_rgb(231, 173, 92)
            } else {
                Color32::from_rgb(157, 183, 218)
            };
            if selected || hit == Some(node) {
                painter.circle_stroke(pos, NODE_RADIUS + 5.0, Stroke::new(1.5, color));
            }
            painter.circle_filled(pos, NODE_RADIUS, Color32::from_rgb(35, 45, 61));
            painter.circle_stroke(pos, NODE_RADIUS, Stroke::new(2.0, color));
            painter.text(
                pos,
                Align2::CENTER_CENTER,
                (node + 1).to_string(),
                FontId::proportional(14.0),
                Color32::WHITE,
            );
            painter.text(
                pos + Vec2::new(0.0, 30.0),
                Align2::CENTER_CENTER,
                format!("{}/3", self.graph.degree(node)),
                FontId::proportional(11.0),
                color,
            );
        }
        if self.graph.nodes.is_empty() && !self.monochrome && !self.background.is_visible() {
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                "Click anywhere to add your first node",
                FontId::proportional(21.0),
                Color32::from_rgb(130, 148, 172),
            );
        }
        if self.panning || response.dragged_by(egui::PointerButton::Middle) {
            response.on_hover_cursor(egui::CursorIcon::Grabbing);
        } else if hit.is_some() || edge_hit.is_some() {
            response.on_hover_cursor(egui::CursorIcon::PointingHand);
        }
    }
}

impl eframe::App for Topograph {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, STORAGE_KEY, &self.graph);
        self.background.save(storage);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.cancel();
        }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Z)) {
            self.undo();
        }
        egui::CentralPanel::default().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Conway Topograph");
                ui.separator();
                ui.label(format!("{} nodes   /   {} edges", self.graph.nodes.len(), self.graph.edges.len()));
                if ui.add_enabled(!self.history.is_empty(), egui::Button::new("Undo")).clicked() { self.undo(); }
                if ui.add_enabled(self.selected.is_some() || self.path_start.is_some(), egui::Button::new("Cancel selection")).clicked() { self.cancel(); }
                if ui.button("Reset view").clicked() { self.pan = Vec2::ZERO; self.zoom = 1.0; }
                if ui.add_enabled(!self.graph.nodes.is_empty(), egui::Button::new("Clear")).clicked() {
                    self.remember();
                    self.graph = Graph::default();
                    self.cancel();
                }
            });
            ui.horizontal(|ui| {
                ui.label("View:");
                ui.selectable_value(&mut self.monochrome, false, "Editor");
                ui.selectable_value(&mut self.monochrome, true, "Black & white");
                ui.label(format!("Zoom: {:.0}%", self.zoom * 100.0));
                ui.separator();
                if ui.button("Background image").clicked() {
                    self.background.open = true;
                }
                if ui.button("Export CeTZ").clicked() {
                    self.export_open = true;
                    self.export_status.clear();
                }
            });
            ui.label("Click empty space: first node only  |  Click node, then endpoint: connect  |  Click edge: single/double  |  Drag node: move");
            ui.label("Ctrl+click two nodes: single/double path  |  Right click node: delete  |  Esc / right click elsewhere: cancel  |  Ctrl/Cmd+Z: undo");
            ui.label("Shift+click: extend from endpoint  |  Wheel: zoom  |  Drag empty space / middle drag: pan  |  Automatically saved");
            ui.colored_label(ANGLE_COLOR, "Red edges: angle of 90 degrees or less at a shared node.");
            if !self.message.is_empty() {
                ui.colored_label(Color32::from_rgb(255, 180, 100), &self.message);
            } else if let Some(node) = self.path_start {
                ui.colored_label(ACCENT, format!("Path from node {}: Ctrl+click the other endpoint.", node + 1));
            } else if let Some(node) = self.selected {
                ui.colored_label(ACCENT, format!("From node {}: click another node or empty space to connect.", node + 1));
            } else {
                ui.label(if self.monochrome { "Ready. Each node accepts up to 3 edges." } else { "Ready. Each node accepts up to 3 edges; amber nodes are full." });
            }
            ui.separator();
            self.canvas(ui);
        });
        self.export_window(ui.ctx());
        self.background.window(ui.ctx());
    }
}

fn main() -> eframe::Result {
    eframe::run_native(
        "Conway Topograph",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1100.0, 760.0])
                .with_min_inner_size([800.0, 480.0]),
            ..Default::default()
        },
        Box::new(|cc| Ok(Box::new(Topograph::new(cc)))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_click_only_creates_first_node_unless_connecting() {
        let mut app = Topograph::default();
        app.click(None, [0.0, 0.0], false);
        let history_len = app.history.len();
        for shift in [false, true] {
            app.click(None, [100.0, 0.0], shift);
            assert_eq!(app.graph.nodes.len(), 1);
            assert_eq!(app.selected, None);
            assert_eq!(app.history.len(), history_len);
        }
        app.click(Some(0), [0.0; 2], false);
        app.click(None, [100.0, 0.0], true);
        assert_eq!(app.graph.edges, vec![[0, 1]]);
        assert_eq!(app.selected, Some(1));
        app.cancel();
        app.click(None, [200.0, 0.0], false);
        assert_eq!(app.graph.nodes.len(), 2);
        app.remove_node(1);
        app.remove_node(0);
        app.click(None, [50.0, 50.0], false);
        assert_eq!(app.graph.nodes, vec![[50.0, 50.0]]);
    }

    #[test]
    fn ctrl_path_toggle_unifies_edges_leaves_branches_and_supports_undo() {
        let mut app = Topograph::default();
        for i in 0..5 {
            app.graph.add_node([i as f32, 0.0]);
        }
        for [a, b] in [[0, 1], [1, 2], [1, 3]] {
            app.graph.connect(a, b).unwrap();
        }
        app.graph.double_edges.extend([0, 2]);
        app.ctrl_click_node(0);
        app.ctrl_click_node(2);
        assert_eq!(app.path_start, None);
        assert_eq!(
            app.graph.double_edges,
            std::collections::BTreeSet::from([0, 1, 2])
        );
        app.ctrl_click_node(0);
        app.ctrl_click_node(2);
        assert_eq!(
            app.graph.double_edges,
            std::collections::BTreeSet::from([2])
        );
        app.undo();
        assert_eq!(
            app.graph.double_edges,
            std::collections::BTreeSet::from([0, 1, 2])
        );
        let history_len = app.history.len();
        app.ctrl_click_node(1); // Full nodes are still selectable for path editing.
        app.ctrl_click_node(1);
        app.ctrl_click_node(4);
        assert_eq!(app.path_start, Some(1));
        assert_eq!(app.history.len(), history_len);
        app.cancel();
        assert_eq!(app.path_start, None);
        assert_eq!(app.graph.edges.len(), 3);
        assert!(app.graph.is_valid());
    }

    #[test]
    fn shift_click_extends_chain_until_normal_click() {
        let mut app = Topograph::default();
        app.click(None, [0.0, 0.0], true);
        assert_eq!(app.selected, Some(0));
        app.click(None, [100.0, 0.0], true);
        assert_eq!(app.selected, Some(1));
        app.click(None, [200.0, 0.0], true);
        assert_eq!(app.selected, Some(2));
        app.click(None, [300.0, 0.0], false);
        assert_eq!(app.selected, None);
        assert_eq!(app.graph.edges, vec![[0, 1], [1, 2], [2, 3]]);
        app.undo();
        assert_eq!(app.graph.nodes.len(), 3);
        assert_eq!(app.graph.edges, vec![[0, 1], [1, 2]]);
        assert!(app.graph.is_valid());
    }

    #[test]
    fn shift_connection_continues_only_if_endpoint_has_capacity() {
        let mut app = Topograph::default();
        for i in 0..4 {
            app.graph.add_node([i as f32 * 100.0, 0.0]);
        }
        app.click(Some(0), [0.0; 2], false);
        app.click(Some(1), [0.0; 2], true);
        assert_eq!(app.selected, Some(1));
        // A duplicate edge must keep the previous start selected.
        app.click(Some(0), [0.0; 2], true);
        assert_eq!(app.selected, Some(1));
        app.click(Some(2), [0.0; 2], false);
        app.click(Some(3), [0.0; 2], false);
        app.click(Some(1), [0.0; 2], true);
        assert_eq!(app.graph.degree(1), 3);
        assert_eq!(app.selected, None);
        assert!(app.graph.is_valid());
    }

    #[test]
    fn deleting_selected_node_cancels_connection_and_undo_restores_graph() {
        let mut app = Topograph::default();
        app.click(None, [10.0, 10.0], false);
        app.click(Some(0), [0.0; 2], false);
        app.click(None, [100.0, 10.0], false);
        app.toggle_edge(0);
        app.click(Some(0), [0.0; 2], false);
        app.remove_node(0);
        assert_eq!(app.selected, None);
        assert_eq!(app.graph.nodes, vec![[100.0, 10.0]]);
        assert!(app.graph.edges.is_empty());
        assert!(app.graph.double_edges.is_empty());
        app.undo();
        assert_eq!(app.graph.nodes, vec![[10.0, 10.0], [100.0, 10.0]]);
        assert_eq!(app.graph.edges, vec![[0, 1]]);
        assert!(app.graph.double_edges.contains(&0));
        assert!(app.graph.is_valid());
    }

    #[test]
    fn clicks_add_connect_and_undo() {
        let mut app = Topograph::default();
        app.click(None, [10.0, 10.0], false);
        app.click(Some(0), [10.0, 10.0], false);
        app.click(None, [100.0, 10.0], false);
        assert_eq!(app.graph.edges, vec![[0, 1]]);
        assert_eq!(app.selected, None);
        app.undo();
        assert_eq!(app.graph.nodes.len(), 1);
        assert!(app.graph.edges.is_empty());
    }

    #[test]
    fn failed_endpoint_keeps_selection_and_full_node_cannot_start() {
        let mut app = Topograph::default();
        for i in 0..5 {
            app.graph.add_node([i as f32 * 100.0, 10.0]);
        }
        for end in 1..=3 {
            app.click(Some(0), [0.0; 2], false);
            app.click(Some(end), [0.0; 2], false);
        }
        app.click(Some(0), [0.0; 2], false);
        assert_eq!(app.selected, None);
        app.click(Some(4), [0.0; 2], false);
        let history_len = app.history.len();
        app.click(Some(0), [0.0; 2], false);
        assert_eq!(app.selected, Some(4));
        assert_eq!(app.graph.edges.len(), 3);
        assert_eq!(app.history.len(), history_len);
        app.cancel();
        assert_eq!(app.selected, None);
    }
}
