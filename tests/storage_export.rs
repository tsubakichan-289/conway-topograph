use conway_topograph::{
    app::App,
    export,
    render::{GraphMode, Options, Palette},
    storage::Saved,
};
use topograph_core::{
    algebra::Form,
    geometry::{Frame, Isometry},
    scene::{Limits, Scene},
    topology::Triangle,
};
#[test]
fn graph_modes_persist_and_export_the_selected_edges() {
    let scene = Scene::generate(
        &Frame::new(Triangle::root()),
        Isometry::default(),
        Limits {
            max_vertices: 20,
            ..Default::default()
        },
    );
    for mode in [GraphMode::Topograph, GraphMode::Dual, GraphMode::Overlay] {
        let mut app = App::default();
        app.graph_mode = mode;
        let json = Saved::capture(&app).to_json().unwrap();
        assert_eq!(Saved::from_json(&json).unwrap().graph_mode, mode);
        let mut legacy: serde_json::Value = serde_json::from_str(&json).unwrap();
        legacy.as_object_mut().unwrap().remove("graph_mode");
        assert_eq!(
            Saved::from_json(&legacy.to_string()).unwrap().graph_mode,
            GraphMode::Topograph
        );
        let options = Options {
            labels: false,
            wells: false,
            rivers: false,
            lakes: false,
            ..app.render_options()
        };
        let svg = export::svg(&scene, &Form::small(1, 0, 1), &options, 400.0, 640).unwrap();
        let expected = match mode {
            GraphMode::Topograph => scene.edges.len(),
            GraphMode::Dual => scene.dual_edges.len(),
            GraphMode::Overlay => scene.edges.len() + scene.dual_edges.len(),
        };
        assert_eq!(svg.matches("stroke-linecap=\"round\"").count(), expected);
        // Clip, background, and disk boundary are the only circles in dual mode.
        assert_eq!(
            svg.matches("<circle").count(),
            3 + if mode.shows_topograph() {
                scene.vertices.len()
            } else {
                0
            }
        );
        assert!(!svg.contains("NaN") && !svg.contains("inf"));
        let image = image::load_from_memory(&export::png(&svg).unwrap()).unwrap();
        assert_eq!((image.width(), image.height()), (640, 640));
    }
}
#[test]
fn json_roundtrip_and_validation() {
    let mut app = App::default();
    app.editor.set_form(Form {
        a: "99999999999999999999999999999999999999".parse().unwrap(),
        b: (-7).into(),
        c: 2.into(),
    });
    let json = Saved::capture(&app).to_json().unwrap();
    let restored = Saved::from_json(&json).unwrap();
    assert_eq!(restored.validate().unwrap().0.form, app.editor.form);
    let mut s = Saved::capture(&app);
    s.fixed[0].value = "0".into();
    assert!(s.validate().is_err());
    let mut s = Saved::capture(&app);
    s.anchor[0] = ["0".into(), "0".into()];
    assert!(s.validate().is_err());
    let mut s = Saved::capture(&app);
    s.zoom = f32::NAN;
    assert!(s.validate().is_err());
    let bad = json.replace("\"version\": 1", "\"version\": 2");
    assert!(Saved::from_json(&bad).is_err());
}
#[test]
fn export_png_and_svg_contain_features_and_are_clipped() {
    let frame = Frame::new(Triangle::root());
    let scene = Scene::generate(
        &frame,
        Isometry::default(),
        Limits {
            max_vertices: 200,
            ..Default::default()
        },
    );
    for (c, highlight) in [(1, "#efc04f"), (-2, "#3788f7"), (-1, "#1b586f")] {
        let svg = export::svg(
            &scene,
            &Form::small(1, 0, c),
            &Options::default(),
            400.0,
            640,
        )
        .unwrap();
        assert!(svg.contains(highlight));
        assert!(svg.contains("clip-path=\"url(#disk)\""));
        assert!(svg.contains(" A "));
        assert!(!svg.contains("NaN"));
        let bytes = export::png(&svg).unwrap();
        let image = image::load_from_memory(&bytes).unwrap();
        assert_eq!((image.width(), image.height()), (640, 640));
        assert!(image.to_rgb8().pixels().any(|p| p[2] > 100 && p[0] != p[2]));
    }
}

#[test]
fn custom_colors_persist_export_and_old_settings_remain_readable() {
    let mut app = App::default();
    app.palette = Palette {
        canvas: [12, 34, 56],
        background: [240, 241, 242],
        edge: [81, 23, 97],
        text: [19, 47, 61],
        boundary: [17, 71, 113],
    };
    let saved = Saved::capture(&app).to_json().unwrap();
    assert_eq!(Saved::from_json(&saved).unwrap().palette, app.palette);
    let mut legacy: serde_json::Value = serde_json::from_str(&saved).unwrap();
    legacy.as_object_mut().unwrap().remove("palette");
    assert_eq!(
        Saved::from_json(&legacy.to_string()).unwrap().palette,
        Palette::default()
    );

    let frame = Frame::new(Triangle::root());
    let scene = Scene::generate(
        &frame,
        Isometry::default(),
        Limits {
            max_vertices: 200,
            ..Default::default()
        },
    );
    let form = Form::small(1, 0, -2);
    let options = app.render_options();
    let svg = export::svg(&scene, &form, &options, 400.0, 640).unwrap();
    assert!(svg.contains("width=\"100%\" height=\"100%\" fill=\"#0c2238\""));
    assert!(svg.contains("stroke=\"#511761\""));
    assert!(svg.contains("fill=\"#132f3d\">"));
    assert!(svg.contains("stroke=\"#114771\""));

    let empty = Scene::generate(
        &frame,
        Isometry::default(),
        Limits {
            max_depth: 0,
            ..Default::default()
        },
    );
    let options = Options {
        labels: false,
        wells: false,
        lakes: false,
        ..options
    };
    let svg = export::svg(&empty, &form, &options, 400.0, 640).unwrap();
    let image = image::load_from_memory(&export::png(&svg).unwrap())
        .unwrap()
        .to_rgb8();
    assert_eq!(image.get_pixel(0, 0).0, app.palette.canvas);
    assert_eq!(image.get_pixel(480, 320).0, app.palette.background);
}
