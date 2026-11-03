use crate::render::{self, Options};
use std::{collections::BTreeMap, fmt::Write, path::Path};
use thiserror::Error;
use topograph_core::{algebra::Form, geometry::Complex, scene::Scene};
#[derive(Debug, Error)]
pub enum ExportError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Invalid SVG: {0}")]
    Svg(#[from] resvg::usvg::Error),
    #[error("Could not allocate the PNG image.")]
    Allocation,
    #[error("Output size must be between 64 and 4096 pixels.")]
    Size,
}
fn color(c: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}
fn coord(z: Complex, r: f64, s: f64) -> (f64, f64) {
    (s / 2.0 + r * z.re, s / 2.0 - r * z.im)
}
fn polygon_path(points: &[Complex], r: f64, s: f64) -> String {
    let area: f64 = points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .take(points.len())
        .map(|(&a, &b)| a.cross(b))
        .sum();
    let ordered: Vec<_> = if area < 0.0 {
        points.iter().rev().collect()
    } else {
        points.iter().collect()
    };
    let vertices = ordered
        .iter()
        .map(|&&z| {
            let (x, y) = coord(z, r, s);
            format!("{x:.5} {y:.5}")
        })
        .collect::<Vec<_>>()
        .join(" L ");
    format!("M {vertices} Z ")
}
/// Exact orthogonal-circle SVG arc (diameter when the endpoints are collinear).
fn arc(a: Complex, b: Complex, r: f64, s: f64) -> String {
    let (x, y) = coord(a, r, s);
    let (u, v) = coord(b, r, s);
    let det = a.cross(b);
    if det.abs() < 1e-12 {
        return format!("M {x:.5} {y:.5} L {u:.5} {v:.5}");
    }
    let ka = (1.0 + a.norm_sq()) / 2.0;
    let kb = (1.0 + b.norm_sq()) / 2.0;
    let center = Complex::new((ka * b.im - kb * a.im) / det, (a.re * kb - b.re * ka) / det);
    let radius = (center.norm_sq() - 1.0).max(0.0).sqrt() * r;
    let sweep = usize::from((a - center).cross(b - center) < 0.0);
    format!("M {x:.5} {y:.5} A {radius:.5} {radius:.5} 0 0 {sweep} {u:.5} {v:.5}")
}
pub fn svg(
    scene: &Scene,
    form: &Form,
    options: &Options,
    source_radius: f64,
    size: u32,
) -> Result<String, ExportError> {
    if !(64..=4096).contains(&size) {
        return Err(ExportError::Size);
    }
    let s = size as f64;
    let r = s * 0.46;
    let scale = r / source_radius;
    let mut out = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{size}\" height=\"{size}\" viewBox=\"0 0 {size} {size}\"><title>Conway topograph: a={}, b={}, c={}, D={}</title><defs><clipPath id=\"disk\"><circle cx=\"{}\" cy=\"{}\" r=\"{r}\"/></clipPath></defs><rect width=\"100%\" height=\"100%\" fill=\"{}\"/><circle cx=\"{}\" cy=\"{}\" r=\"{r}\" fill=\"{}\"/><g clip-path=\"url(#disk)\">",
        form.a,
        form.b,
        form.c,
        form.discriminant(),
        s / 2.0,
        s / 2.0,
        color(options.palette.canvas),
        s / 2.0,
        s / 2.0,
        color(options.palette.background)
    );
    let wells = render::well_faces(scene, form);
    // Fill each face as one compound path so its internal patch boundaries do not
    // leave anti-aliasing seams in PNG/SVG, especially on saturated backgrounds.
    let mut fills = BTreeMap::new();
    for p in &scene.patches {
        if let Some(c) = render::patch_color(&p.face, form, options, &wells) {
            let (_, path) = fills.entry(&p.face).or_insert_with(|| (c, String::new()));
            path.push_str(&polygon_path(&p.polygon, r, s));
        }
    }
    for (c, path) in fills.values() {
        write!(out, "<path d=\"{path}\" fill=\"{}\"/>", color(*c)).unwrap();
    }
    for (e, c, width) in render::graph_edges(scene, form, options) {
        if let (Some(&a), Some(&b)) = (e.path.first(), e.path.last()) {
            write!(out,"<path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\" stroke-linecap=\"round\"/>",arc(a,b,r,s),color(c),width as f64*scale).unwrap();
        }
    }
    for v in scene
        .vertices
        .iter()
        .filter(|_| options.graph_mode.shows_topograph())
    {
        let well = options.wells && topograph_core::features::well(form, &v.triangle).is_some();
        let (x, y) = coord(v.center, r, s);
        write!(
            out,
            "<circle cx=\"{x}\" cy=\"{y}\" r=\"{}\" fill=\"{}\"/>",
            if well { 4.0 * scale } else { 1.4 * scale },
            color(if well {
                render::GOLD
            } else {
                options.palette.edge
            })
        )
        .unwrap();
    }
    for l in render::labels_for(scene, form, options, source_radius) {
        let (x, y) = coord(l.position, r, s);
        write!(out,"<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"{}\"/><text x=\"{x}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"{}\" fill=\"{}\">{}</text>",x-l.width*r/2.0,y-l.height*r/2.0,l.width*r,l.height*r,3.0*scale,color(l.background),y+5.0*scale,14.0*scale,color(l.color),l.text).unwrap();
    }
    write!(out,"</g><circle cx=\"{}\" cy=\"{}\" r=\"{r}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"/></svg>",s/2.0,s/2.0,color(options.palette.boundary),2.0*scale).unwrap();
    Ok(out)
}
pub fn save_svg(path: &Path, data: &str) -> Result<(), ExportError> {
    std::fs::write(path, data)?;
    Ok(())
}
pub fn png(data: &str) -> Result<Vec<u8>, ExportError> {
    let mut options = resvg::usvg::Options {
        font_family: "monospace".to_owned(),
        ..Default::default()
    };
    options.fontdb_mut().load_system_fonts();
    // Include the same monospace font used by egui, without depending on installed fonts.
    options.fontdb_mut().load_font_data(
        eframe::egui::FontDefinitions::default()
            .font_data
            .get("Hack")
            .map(|f| f.font.to_vec())
            .unwrap_or_default(),
    );
    options.fontdb_mut().set_monospace_family("Hack");
    let tree = resvg::usvg::Tree::from_str(data, &options)?;
    let size = tree.size().to_int_size();
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size.width(), size.height())
        .ok_or(ExportError::Allocation)?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    pixmap
        .encode_png()
        .map_err(|e| ExportError::Io(std::io::Error::other(e)))
}
pub fn save_png(path: &Path, data: &str) -> Result<(), ExportError> {
    std::fs::write(path, png(data)?)?;
    Ok(())
}
