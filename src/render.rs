use eframe::egui::{self, Align2, Color32, FontId, Pos2, Stroke};
use num_bigint::BigInt;
use num_traits::{Signed, ToPrimitive, Zero};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use topograph_core::{
    algebra::{Form, Primitive},
    features,
    geometry::{Complex, polygon_contains},
    scene::Scene,
};
#[derive(Clone, Copy)]
pub struct Disk {
    pub center: Pos2,
    pub radius: f32,
}
impl Disk {
    pub fn screen(self, z: Complex) -> Pos2 {
        self.center + egui::vec2(z.re as f32, -z.im as f32) * self.radius
    }
    pub fn point(self, p: Pos2) -> Complex {
        let v = (p - self.center) / self.radius;
        Complex::new(v.x as f64, -v.y as f64)
    }
}
#[derive(Clone, Debug)]
pub struct Options {
    pub labels: bool,
    pub rivers: bool,
    pub wells: bool,
    pub lakes: bool,
    pub selected: Option<Primitive>,
    pub palette: Palette,
    pub color_mode: ColorMode,
    pub graph_mode: GraphMode,
    pub value_scale: u32,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            labels: true,
            rivers: true,
            wells: true,
            lakes: true,
            selected: None,
            palette: Palette::default(),
            color_mode: ColorMode::default(),
            graph_mode: GraphMode::default(),
            value_scale: default_value_scale(),
        }
    }
}
#[derive(Clone, Debug)]
pub struct Label {
    pub face: Primitive,
    pub text: String,
    pub position: Complex,
    pub width: f64,
    pub height: f64,
    pub color: [u8; 3],
    pub background: [u8; 3],
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColorMode {
    #[default]
    Normal,
    ValueColors,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphMode {
    #[default]
    Topograph,
    Dual,
    Overlay,
}
impl GraphMode {
    pub fn shows_topograph(self) -> bool {
        self != Self::Dual
    }
    pub fn shows_dual(self) -> bool {
        self != Self::Topograph
    }
}
/// Shared edge selection and styling for the canvas and image exports.
pub fn graph_edges<'a>(
    scene: &'a Scene,
    form: &'a Form,
    options: &'a Options,
) -> impl Iterator<Item = (&'a topograph_core::scene::Edge, [u8; 3], f32)> + 'a {
    scene
        .dual_edges
        .iter()
        .filter(|_| options.graph_mode.shows_dual())
        .map(|e| {
            let (color, width) = if options.graph_mode == GraphMode::Overlay {
                (options.palette.boundary, 1.0)
            } else {
                edge_style(e, form, options)
            };
            (e, color, width)
        })
        .chain(
            scene
                .edges
                .iter()
                .filter(|_| options.graph_mode.shows_topograph())
                .map(|e| {
                    let (color, width) = edge_style(e, form, options);
                    (e, color, width)
                }),
        )
}
pub fn default_value_scale() -> u32 {
    16
}
/// Channel intensity = round(255 * scale / (scale + |Q|)).
/// BigInt keeps even enormous labels safe; only the resulting byte is converted.
pub fn value_color(value: &BigInt, scale: u32) -> [u8; 3] {
    if value.is_zero() {
        return CYAN;
    }
    let scale = BigInt::from(scale.max(1));
    let denominator = value.abs() + &scale;
    let numerator = BigInt::from(255) * scale;
    let intensity = ((numerator + &denominator / 2u32) / denominator)
        .to_u8()
        .expect("bounded color channel");
    if value.is_positive() {
        [intensity, 0, 0]
    } else {
        [0, 0, intensity]
    }
}
fn luminance(color: [u8; 3]) -> f64 {
    let [r, g, b] = color.map(|c| {
        let c = c as f64 / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    });
    0.2126 * r + 0.7152 * g + 0.0722 * b
}
fn readable_text(text: [u8; 3], background: [u8; 3]) -> [u8; 3] {
    let a = luminance(text);
    let b = luminance(background);
    if (a.max(b) + 0.05) / (a.min(b) + 0.05) >= 4.5 {
        text
    } else if b > 0.179 {
        [0, 0, 0]
    } else {
        [255, 255, 255]
    }
}
pub const BACKGROUND: [u8; 3] = [15, 23, 35];
pub const GOLD: [u8; 3] = [239, 192, 79];
pub const BLUE: [u8; 3] = [55, 136, 247];
pub const CYAN: [u8; 3] = [103, 217, 238];
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Palette {
    pub canvas: [u8; 3],
    pub background: [u8; 3],
    pub edge: [u8; 3],
    pub text: [u8; 3],
    pub boundary: [u8; 3],
}
impl Default for Palette {
    fn default() -> Self {
        Self {
            canvas: [9, 14, 23],
            background: BACKGROUND,
            edge: [113, 135, 161],
            text: [222, 231, 242],
            boundary: [143, 168, 192],
        }
    }
}
pub fn rgb(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}
pub fn well_faces(scene: &Scene, form: &Form) -> BTreeSet<Primitive> {
    scene
        .vertices
        .iter()
        .filter(|v| features::well(form, &v.triangle).is_some())
        .flat_map(|v| v.triangle.faces().clone())
        .collect()
}
pub fn patch_color(
    face: &Primitive,
    form: &Form,
    options: &Options,
    wells: &BTreeSet<Primitive>,
) -> Option<[u8; 3]> {
    if options.color_mode == ColorMode::ValueColors {
        let value = form.evaluate(face);
        Some(if value.is_zero() && !options.lakes {
            options.palette.background
        } else {
            value_color(&value, options.value_scale)
        })
    } else if options.lakes && features::lake(form, face) {
        Some([27, 88, 111])
    } else if options.selected.as_ref() == Some(face) {
        Some([65, 67, 117])
    } else if options.wells && wells.contains(face) {
        Some([61, 48, 24])
    } else {
        None
    }
}
pub fn edge_style(
    edge: &topograph_core::scene::Edge,
    form: &Form,
    options: &Options,
) -> ([u8; 3], f32) {
    if options.rivers && features::river(form, &edge.faces) {
        (BLUE, 3.6)
    } else if options.wells
        && features::well(form, &edge.from).is_some()
        && features::well(form, &edge.to).is_some()
    {
        (GOLD, 3.2)
    } else if options.color_mode == ColorMode::ValueColors
        && options
            .selected
            .as_ref()
            .is_some_and(|face| edge.faces.contains(face))
    {
        (options.palette.boundary, 2.4)
    } else {
        (options.palette.edge, 1.0)
    }
}
pub fn labels_for(scene: &Scene, form: &Form, options: &Options, radius: f64) -> Vec<Label> {
    let wells = well_faces(scene, form);
    let mut placed = BTreeSet::new();
    let mut result: Vec<Label> = Vec::new();
    let mut patches: Vec<_> = scene.patches.iter().collect();
    patches.sort_by(|a, b| b.clearance.total_cmp(&a.clearance));
    for p in patches {
        let is_well = options.wells && wells.contains(&p.face);
        let is_lake = options.lakes && features::lake(form, &p.face);
        if (!options.labels && !is_well && !is_lake)
            || placed.contains(&p.face)
            || p.clearance < 9.0
        {
            continue;
        }
        let text = form.evaluate(&p.face).to_string();
        let width = (text.len() as f64 * 8.5 + 8.0) / radius;
        let height = 22.0 / radius;
        let corners = [
            Complex::new(-width / 2.0, -height / 2.0),
            Complex::new(width / 2.0, -height / 2.0),
            Complex::new(width / 2.0, height / 2.0),
            Complex::new(-width / 2.0, height / 2.0),
        ]
        .map(|d| p.label + d);
        if corners
            .iter()
            .any(|&z| z.norm() > 0.975 || !polygon_contains(&p.polygon, z))
        {
            continue;
        }
        if result.iter().any(|l| {
            (l.position.re - p.label.re).abs() < (l.width + width) / 2.0
                && (l.position.im - p.label.im).abs() < (l.height + height) / 2.0
        }) {
            continue;
        }
        let background = if options.color_mode == ColorMode::ValueColors {
            patch_color(&p.face, form, options, &wells).unwrap_or(options.palette.background)
        } else {
            options.palette.background
        };
        let text_color = if is_lake {
            CYAN
        } else if is_well {
            GOLD
        } else {
            options.palette.text
        };
        result.push(Label {
            face: p.face.clone(),
            text,
            position: p.label,
            width,
            height,
            color: if options.color_mode == ColorMode::ValueColors {
                readable_text(text_color, background)
            } else {
                text_color
            },
            background,
        });
        placed.insert(p.face.clone());
    }
    result
}
pub fn paint(painter: &egui::Painter, scene: &Scene, form: &Form, disk: Disk, options: &Options) {
    painter.circle_filled(disk.center, disk.radius, rgb(options.palette.background));
    let wells = well_faces(scene, form);
    for p in &scene.patches {
        if let Some(color) = patch_color(&p.face, form, options, &wells) {
            let mut mesh = egui::Mesh::default();
            for &z in &p.polygon {
                mesh.colored_vertex(disk.screen(z), rgb(color));
            }
            for &[a, b, c] in &p.triangles {
                mesh.add_triangle(a as u32, b as u32, c as u32);
            }
            painter.add(egui::Shape::mesh(mesh));
        }
    }
    for (e, color, width) in graph_edges(scene, form, options) {
        let safe = 1.0 - (width as f64 / 2.0 + 1.0) / disk.radius as f64;
        let points: Vec<_> = e
            .path
            .iter()
            .map(|&z| disk.screen(z.bounded(safe.max(0.0))))
            .collect();
        if points.len() > 1 {
            painter.add(egui::Shape::line(points, Stroke::new(width, rgb(color))));
        }
    }
    for v in scene
        .vertices
        .iter()
        .filter(|_| options.graph_mode.shows_topograph())
    {
        let is_well = options.wells && features::well(form, &v.triangle).is_some();
        let radius = if is_well { 4.0 } else { 1.4 };
        if v.center.norm() + (radius as f64 + 1.0) / (disk.radius as f64) < 1.0 {
            painter.circle_filled(
                disk.screen(v.center),
                radius,
                if is_well {
                    rgb(GOLD)
                } else {
                    rgb(options.palette.edge)
                },
            );
        }
    }
    for l in labels_for(scene, form, options, disk.radius as f64) {
        let rect = egui::Rect::from_center_size(
            disk.screen(l.position),
            egui::vec2(l.width as f32, l.height as f32) * disk.radius,
        );
        painter.rect_filled(rect, 3.0, rgb(l.background));
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            l.text,
            FontId::monospace(14.0),
            rgb(l.color),
        );
    }
    painter.circle_stroke(
        disk.center,
        disk.radius,
        Stroke::new(2.0, rgb(options.palette.boundary)),
    );
}
