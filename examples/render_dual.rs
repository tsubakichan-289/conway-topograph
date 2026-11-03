use conway_topograph::{
    export,
    render::{GraphMode, Options},
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
        .unwrap_or("/tmp/topograph-dual".into());
    std::fs::create_dir_all(&output)?;
    let scene = Scene::generate(
        &Frame::new(Triangle::root()),
        Isometry::default(),
        Limits::default(),
    );
    for (name, graph_mode) in [
        ("topograph", GraphMode::Topograph),
        ("dual", GraphMode::Dual),
        ("overlay", GraphMode::Overlay),
    ] {
        let options = Options {
            graph_mode,
            labels: false,
            rivers: false,
            wells: false,
            lakes: false,
            ..Default::default()
        };
        let svg = export::svg(&scene, &Form::small(1, 0, 1), &options, 400.0, 1000)?;
        let path = std::path::Path::new(&output).join(name);
        export::save_svg(&path.with_extension("svg"), &svg)?;
        export::save_png(&path.with_extension("png"), &svg)?;
    }
    println!("Wrote topograph, dual, and overlay images to {output}");
    Ok(())
}
