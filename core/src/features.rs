use crate::{
    algebra::{Form, FormKind, Primitive},
    topology::Triangle,
};
use num_bigint::BigInt;
use num_traits::{Signed, Zero};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WellKind {
    Single,
    Double,
}
#[derive(Clone, Debug)]
pub struct Well {
    pub kind: WellKind,
    pub negative: bool,
    pub values: [BigInt; 3],
    /// Actual changes of Q in the three OUTWARD directions.
    pub deltas: [BigInt; 3],
    pub flat_edge: Option<usize>,
}
pub fn river(form: &Form, faces: &[Primitive; 2]) -> bool {
    if form.kind() != FormKind::Indefinite {
        return false;
    }
    let a = form.evaluate(&faces[0]);
    let b = form.evaluate(&faces[1]);
    !a.is_zero() && !b.is_zero() && a.sign() != b.sign()
}
pub fn lake(form: &Form, face: &Primitive) -> bool {
    form.evaluate(face).is_zero()
}
pub fn changes(form: &Form, t: &Triangle) -> [BigInt; 3] {
    let values = t.faces().clone().map(|v| form.evaluate(&v));
    std::array::from_fn(|i| {
        BigInt::from(2) * (&values[(i + 1) % 3] + &values[(i + 2) % 3] - &values[i])
    })
}
pub fn well(form: &Form, t: &Triangle) -> Option<Well> {
    let negative = match form.kind() {
        FormKind::PositiveDefinite => false,
        FormKind::NegativeDefinite => true,
        _ => return None,
    };
    let deltas = changes(form, t);
    if deltas.iter().any(|v| {
        if negative {
            v.is_positive()
        } else {
            v.is_negative()
        }
    }) {
        return None;
    }
    let flat_edge = deltas.iter().position(Zero::is_zero);
    Some(Well {
        kind: if flat_edge.is_some() {
            WellKind::Double
        } else {
            WellKind::Single
        },
        negative,
        values: t.faces().clone().map(|v| form.evaluate(&v)),
        deltas,
        flat_edge,
    })
}
pub fn river_neighbors(form: &Form, t: &Triangle) -> Vec<Triangle> {
    (0..3)
        .filter(|&i| river(form, &t.edge_faces(i)))
        .map(|i| t.neighbor(i))
        .collect()
}
/// Exact descent until a well, river bank, or lake is reached. Bounded work, no global tree expansion.
pub fn seek_landmark(form: &Form, mut t: Triangle, limit: usize) -> Result<Triangle, &'static str> {
    for _ in 0..limit {
        if well(form, &t).is_some()
            || t.faces().iter().any(|v| lake(form, v))
            || !river_neighbors(form, &t).is_empty()
        {
            return Ok(t);
        }
        let sign = form.evaluate(&t.faces()[0]).sign();
        let delta = changes(form, &t);
        let i = (0..3)
            .min_by_key(|&i| {
                if sign == num_bigint::Sign::Minus {
                    -&delta[i]
                } else {
                    delta[i].clone()
                }
            })
            .unwrap();
        let decreases = if sign == num_bigint::Sign::Minus {
            delta[i].is_positive()
        } else {
            delta[i].is_negative()
        };
        if !decreases {
            return Err("No descending edge at this vertex.");
        }
        t = t.neighbor(i);
    }
    Err("Exact descent reached its step limit; select a nearer vertex and retry.")
}
