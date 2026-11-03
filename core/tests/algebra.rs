use num_bigint::BigInt;
use std::collections::BTreeSet;
use topograph_core::{
    algebra::{Form, FormKind, MathError, Primitive},
    topology::Triangle,
};

#[test]
fn evaluation_and_discriminant() {
    for (c, d, kind) in [
        (1, -4, FormKind::PositiveDefinite),
        (2, -8, FormKind::PositiveDefinite),
        (-1, 4, FormKind::Indefinite),
        (-2, 8, FormKind::Indefinite),
    ] {
        let f = Form::small(1, 0, c);
        assert_eq!(f.evaluate(&Primitive::small(3, 2)), BigInt::from(9 + 4 * c));
        assert_eq!(f.discriminant(), BigInt::from(d));
        assert_eq!(f.kind(), kind);
    }
    assert_eq!(Form::small(-1, 0, -2).kind(), FormKind::NegativeDefinite);
    assert_eq!(Form::small(0, 0, 0).kind(), FormKind::Degenerate);
    let n = BigInt::from(10).pow(150);
    let f = Form {
        a: n.clone(),
        b: -&n,
        c: n.clone(),
    };
    assert_eq!(f.evaluate_xy(&n, &n), n.pow(3));
}
#[test]
fn primitive_normalization_and_unimodularity() {
    assert_eq!(Primitive::small(-1, -2), Primitive::small(1, 2));
    assert_eq!(Primitive::small(-1, 0), Primitive::small(1, 0));
    for (p, q) in [(0, 0), (2, 4), (3, 0)] {
        assert_eq!(
            Primitive::new(p.into(), q.into()),
            Err(MathError::NotPrimitive)
        );
    }
    assert!(Primitive::small(1, 0).adjacent(&Primitive::small(8, 1)));
    assert!(!Primitive::small(1, 0).adjacent(&Primitive::small(1, 2)));
}
#[test]
fn propagation_and_tree() {
    let mut seen = BTreeSet::new();
    let mut stack = vec![(Triangle::root(), None, 0)];
    let forms = [
        Form::small(1, 0, 1),
        Form::small(1, 0, 2),
        Form::small(1, 0, -1),
        Form::small(1, 0, -2),
        Form::small(7, -19, 0),
    ];
    while let Some((t, parent, depth)) = stack.pop() {
        assert!(seen.insert(t.clone()), "dual graph has no cycles");
        for i in 0..3 {
            let next = t.neighbor(i);
            assert!(next.neighbors().contains(&t));
            let [u, v] = t.edge_faces(i);
            let other = next.faces().iter().find(|x| **x != u && **x != v).unwrap();
            for f in &forms {
                let u_value = f.evaluate(&u);
                let v_value = f.evaluate(&v);
                let cross = f.evaluate(&t.faces()[i]) - &u_value - &v_value;
                assert_eq!(
                    &cross * &cross - BigInt::from(4) * u_value * v_value,
                    f.discriminant()
                );
                assert_eq!(
                    f.propagated(&u, &v, &f.evaluate(&t.faces()[i])),
                    f.evaluate(other)
                );
            }
            if depth < 5 && Some(&next) != parent.as_ref() {
                stack.push((next, Some(t.clone()), depth + 1));
            }
        }
    }
    assert_eq!(seen.len(), 1 + 3 * (2usize.pow(5) - 1));
}
#[test]
fn restoration_is_exact_and_rejects_invalid_constraints() {
    let f = Form::small(7, -13, 29);
    assert_eq!(Form::from_initial(7.into(), 29.into(), 23.into()), f);
    for vs in [
        [
            Primitive::small(1, 0),
            Primitive::small(0, 1),
            Primitive::small(1, 1),
        ],
        [
            Primitive::small(2, 1),
            Primitive::small(-1, 3),
            Primitive::small(5, 2),
        ],
    ] {
        assert_eq!(
            Form::restore(&vs.map(|v| {
                let q = f.evaluate(&v);
                (v, q)
            }))
            .unwrap(),
            f
        );
    }
    let u = Primitive::small(1, 0);
    assert_eq!(
        Form::restore(&[
            (u.clone(), 1.into()),
            (u, 1.into()),
            (Primitive::small(0, 1), 1.into())
        ]),
        Err(MathError::Dependent)
    );
    assert_eq!(
        Form::restore(&[
            (Primitive::small(1, 0), 0.into()),
            (Primitive::small(0, 1), 0.into()),
            (Primitive::small(1, 2), 1.into())
        ]),
        Err(MathError::NonIntegral)
    );
}
