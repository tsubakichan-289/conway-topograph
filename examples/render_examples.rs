use conway_topograph::{export, render::Options};
use topograph_core::{
    algebra::Form,
    geometry::{Frame, Isometry},
    scene::{Limits, Scene},
    topology::Triangle,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args()
        .nth(1)
        .unwrap_or("/tmp/topograph-examples".into());
    std::fs::create_dir_all(&output)?;
    let frame = Frame::new(Triangle::root());
    let scene = Scene::generate(&frame, Isometry::default(), Limits::default());
    for (name, c) in [("well-1", 1), ("well-2", 2), ("lake", -1), ("river", -2)] {
        let svg = export::svg(
            &scene,
            &Form::small(1, 0, c),
            &Options::default(),
            400.0,
            1000,
        )?;
        export::save_svg(
            &std::path::Path::new(&output).join(format!("{name}.svg")),
            &svg,
        )?;
        export::save_png(
            &std::path::Path::new(&output).join(format!("{name}.png")),
            &svg,
        )?;
    }
    println!(
        "{} vertices, {} edges; wrote examples to {output}",
        scene.vertices.len(),
        scene.edges.len()
    );
    Ok(())
}
