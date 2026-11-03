use conway_topograph::{
    export,
    render::{ColorMode, Options},
};
use topograph_core::{
    algebra::Form,
    geometry::{Frame, Isometry},
    scene::{Limits, Scene},
    topology::Triangle,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args()
        .nth(1)
        .unwrap_or("/tmp/topograph-value-colors".into());
    std::fs::create_dir_all(&output)?;
    let frame = Frame::new(Triangle::root());
    let scene = Scene::generate(&frame, Isometry::default(), Limits::default());
    let options = Options {
        color_mode: ColorMode::ValueColors,
        ..Default::default()
    };
    let svg = export::svg(&scene, &Form::small(1, 0, -2), &options, 400.0, 1000)?;
    export::save_svg(
        &std::path::Path::new(&output).join("value-colors.svg"),
        &svg,
    )?;
    export::save_png(
        &std::path::Path::new(&output).join("value-colors.png"),
        &svg,
    )?;
    println!("Wrote value-color example to {output}");
    Ok(())
}
