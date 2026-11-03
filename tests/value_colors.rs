use conway_topograph::{
    app::App,
    export,
    render::{self, ColorMode, Options},
    storage::Saved,
};
use num_bigint::BigInt;
use std::collections::BTreeSet;
use topograph_core::{
    algebra::{Form, Primitive},
    geometry::{Frame, Isometry},
    scene::{Limits, Scene},
    topology::Triangle,
};

#[test]
fn signed_magnitudes_fade_monotonically_and_handle_huge_integers() {
    assert_eq!(render::value_color(&1.into(), 1_000_000), [255, 0, 0]);
    assert_eq!(render::value_color(&(-1).into(), 1_000_000), [0, 0, 255]);
    assert_eq!(render::value_color(&16.into(), 16), [128, 0, 0]);
    let mut previous = 255;
    for magnitude in 1..=10000 {
        let positive = render::value_color(&magnitude.into(), 16);
        let negative = render::value_color(&(-magnitude).into(), 16);
        assert_eq!((positive[1], positive[2]), (0, 0));
        assert_eq!(negative, [0, 0, positive[0]]);
        assert!(positive[0] <= previous);
        previous = positive[0];
    }
    let enormous = BigInt::from(10).pow(10000);
    assert_eq!(render::value_color(&enormous, 1_000_000), [0, 0, 0]);
    assert_eq!(render::value_color(&-enormous, 1_000_000), [0, 0, 0]);
    assert_eq!(render::value_color(&0.into(), 16), render::CYAN);
}

#[test]
fn face_colors_preserve_sign_even_on_wells_and_selections() {
    let form = Form::small(1, 0, 1);
    let selected = Primitive::small(1, 0);
    let wells = BTreeSet::from([selected.clone()]);
    let options = Options {
        color_mode: ColorMode::ValueColors,
        selected: Some(selected.clone()),
        ..Default::default()
    };
    assert_eq!(
        render::patch_color(&selected, &form, &options, &wells),
        Some([240, 0, 0])
    );
    let negative = Form::small(-1, 0, -1);
    assert_eq!(
        render::patch_color(&selected, &negative, &options, &wells),
        Some([0, 0, 240])
    );
    let zero = Form::small(0, 0, 0);
    assert_eq!(
        render::patch_color(&selected, &zero, &options, &wells),
        Some(render::CYAN)
    );
    let options = Options {
        lakes: false,
        ..options
    };
    assert_eq!(
        render::patch_color(&selected, &zero, &options, &wells),
        Some(options.palette.background)
    );
}

#[test]
fn mode_roundtrips_and_png_exports_red_blue_and_dark_faces() {
    let mut app = App::default();
    app.color_mode = ColorMode::ValueColors;
    app.value_scale = 16;
    let saved = Saved::capture(&app).to_json().unwrap();
    let restored = Saved::from_json(&saved).unwrap();
    assert_eq!(restored.color_mode, ColorMode::ValueColors);
    assert_eq!(restored.value_scale, 16);
    let mut old: serde_json::Value = serde_json::from_str(&saved).unwrap();
    old.as_object_mut().unwrap().remove("color_mode");
    old.as_object_mut().unwrap().remove("value_scale");
    let old = Saved::from_json(&old.to_string()).unwrap();
    assert_eq!(old.color_mode, ColorMode::Normal);
    assert_eq!(old.value_scale, 16);
    let mut invalid = Saved::capture(&app);
    invalid.value_scale = 0;
    assert!(invalid.validate().is_err());

    let frame = Frame::new(Triangle::root());
    let scene = Scene::generate(
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
        rivers: false,
        ..app.render_options()
    };
    let form = Form::small(1, 0, -1);
    let svg = export::svg(&scene, &form, &options, 400.0, 640).unwrap();
    assert!(svg.contains("fill=\"#f00000\""));
    assert!(svg.contains("fill=\"#0000f0\""));
    let image = image::load_from_memory(&export::png(&svg).unwrap())
        .unwrap()
        .to_rgb8();
    for (face, color) in [
        (Primitive::small(1, 0), [240, 0, 0]),
        (Primitive::small(0, 1), [0, 0, 240]),
        (Primitive::small(1, 1), render::CYAN),
    ] {
        let p = scene.patches.iter().find(|p| p.face == face).unwrap().label;
        let pixel = image.get_pixel(
            (320.0 + 294.4 * (p.re + 0.05)).round() as u32,
            (320.0 - 294.4 * p.im).round() as u32,
        );
        assert_eq!(pixel.0, color);
    }
    let massive = Form {
        a: BigInt::from(10).pow(1000),
        b: 0.into(),
        c: 1.into(),
    };
    let svg = export::svg(&scene, &massive, &options, 400.0, 640).unwrap();
    let image = image::load_from_memory(&export::png(&svg).unwrap())
        .unwrap()
        .to_rgb8();
    let p = scene
        .patches
        .iter()
        .find(|p| p.face == Primitive::small(1, 0))
        .unwrap()
        .label;
    assert_eq!(
        image
            .get_pixel(
                (320.0 + 294.4 * p.re).round() as u32,
                (320.0 - 294.4 * p.im).round() as u32
            )
            .0,
        [0, 0, 0]
    );
}
