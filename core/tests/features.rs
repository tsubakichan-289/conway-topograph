use topograph_core::{
    algebra::{Form, Primitive},
    features::*,
    topology::Triangle,
};
#[test]
fn wells_use_all_three_directions_including_double_wells() {
    for f in [
        Form::small(1, 0, 1),
        Form::small(1, 0, 2),
        Form::small(-1, 0, -2),
    ] {
        let root = Triangle::root();
        let w = well(&f, &root).unwrap();
        assert_eq!(w.kind, WellKind::Double);
        let other = root.neighbor(w.flat_edge.unwrap());
        assert_eq!(well(&f, &other).unwrap().kind, WellKind::Double);
        for i in 0..3 {
            if Some(i) != w.flat_edge {
                assert!(well(&f, &root.neighbor(i)).is_none());
            }
        }
        let far = root.neighbor(0).neighbor(1).neighbor(2);
        assert!(well(&f, &seek_landmark(&f, far, 100).unwrap()).is_some());
    }
    assert_eq!(
        well(
            &Form::small(1, 1, 1),
            &seek_landmark(&Form::small(1, 1, 1), Triangle::root(), 100).unwrap()
        )
        .unwrap()
        .kind,
        WellKind::Single
    );
    assert!(well(&Form::small(1, 0, -2), &Triangle::root()).is_none());
}
#[test]
fn rivers_exclude_zero_and_distinguish_square_discriminant() {
    let f = Form::small(1, 0, -2);
    let root = Triangle::root();
    assert!(!f.square_discriminant());
    assert_eq!(river_neighbors(&f, &root).len(), 2);
    let mut previous = None;
    let mut t = root;
    for _ in 0..64 {
        let next = river_neighbors(&f, &t)
            .into_iter()
            .find(|n| Some(n) != previous.as_ref())
            .unwrap();
        assert_eq!(river_neighbors(&f, &next).len(), 2);
        assert!(!next.faces().iter().any(|v| lake(&f, v)));
        previous = Some(t);
        t = next;
    }
    let f = Form::small(1, 0, -1);
    assert!(f.square_discriminant());
    assert!(lake(&f, &Primitive::small(1, 1)));
    assert!(lake(&f, &Primitive::small(-1, 1)));
    let root = Triangle::root();
    let bank = river_neighbors(&f, &root);
    assert_eq!(bank.len(), 1);
    assert_eq!(river_neighbors(&f, &bank[0]), vec![root]);
    assert!(!river(
        &f,
        &[Primitive::small(1, 1), Primitive::small(1, 0)]
    ));
    let f = Form::small(0, 1, 0);
    assert!(f.square_discriminant());
    assert!(river_neighbors(&f, &Triangle::root()).is_empty());
}
#[test]
fn degenerate_and_zero_forms_have_lakes_but_no_wells_or_rivers() {
    for f in [
        Form::small(1, 0, 0),
        Form::small(-1, 0, 0),
        Form::small(0, 0, 0),
    ] {
        assert!(lake(&f, &Primitive::small(0, 1)));
        assert!(well(&f, &Triangle::root()).is_none());
        assert!(river_neighbors(&f, &Triangle::root()).is_empty());
    }
}
