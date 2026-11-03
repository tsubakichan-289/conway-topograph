//! Floating-point drawing coordinates only; integer geometry lives in Frame/Triangle.
use crate::{algebra::Primitive, topology::Triangle};
use num_bigint::BigInt;
use num_traits::{Signed, ToPrimitive};
use serde::{Deserialize, Serialize};
use std::ops::{Add, Div, Mul, Neg, Sub};

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Complex {
    pub re: f64,
    pub im: f64,
}
impl Complex {
    pub const ZERO: Self = Self { re: 0.0, im: 0.0 };
    pub const ONE: Self = Self { re: 1.0, im: 0.0 };
    pub const I: Self = Self { re: 0.0, im: 1.0 };
    pub fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }
    pub fn conj(self) -> Self {
        Self::new(self.re, -self.im)
    }
    pub fn norm_sq(self) -> f64 {
        self.re * self.re + self.im * self.im
    }
    pub fn norm(self) -> f64 {
        self.norm_sq().sqrt()
    }
    pub fn finite(self) -> bool {
        self.re.is_finite() && self.im.is_finite()
    }
    pub fn dot(self, other: Self) -> f64 {
        self.re * other.re + self.im * other.im
    }
    pub fn cross(self, other: Self) -> f64 {
        self.re * other.im - self.im * other.re
    }
    pub fn unit(self) -> Self {
        self / self.norm()
    }
    pub fn bounded(self, radius: f64) -> Self {
        if self.norm() > radius {
            self.unit() * radius
        } else {
            self
        }
    }
}
impl Add for Complex {
    type Output = Self;
    fn add(self, v: Self) -> Self {
        Self::new(self.re + v.re, self.im + v.im)
    }
}
impl Sub for Complex {
    type Output = Self;
    fn sub(self, v: Self) -> Self {
        Self::new(self.re - v.re, self.im - v.im)
    }
}
impl Neg for Complex {
    type Output = Self;
    fn neg(self) -> Self {
        Self::new(-self.re, -self.im)
    }
}
impl Mul for Complex {
    type Output = Self;
    fn mul(self, v: Self) -> Self {
        Self::new(
            self.re * v.re - self.im * v.im,
            self.re * v.im + self.im * v.re,
        )
    }
}
impl Div for Complex {
    type Output = Self;
    fn div(self, v: Self) -> Self {
        self * v.conj() / v.norm_sq()
    }
}
impl Mul<f64> for Complex {
    type Output = Self;
    fn mul(self, v: f64) -> Self {
        Self::new(self.re * v, self.im * v)
    }
}
impl Div<f64> for Complex {
    type Output = Self;
    fn div(self, v: f64) -> Self {
        Self::new(self.re / v, self.im / v)
    }
}

pub fn cayley(z: Complex) -> Complex {
    (z - Complex::I) / (z + Complex::I)
}
pub fn inverse_cayley(w: Complex) -> Complex {
    Complex::I * (Complex::ONE + w) / (Complex::ONE - w)
}
pub fn root_center() -> Complex {
    cayley(Complex::new(0.5, 3f64.sqrt() / 2.0))
}

/// Compact canonical representation avoids an ever-growing product of matrices.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Isometry {
    alpha: Complex,
    rotation: Complex,
}
impl Default for Isometry {
    fn default() -> Self {
        Self {
            alpha: Complex::ZERO,
            rotation: Complex::ONE,
        }
    }
}
impl Isometry {
    pub fn shift(alpha: Complex) -> Option<Self> {
        (alpha.finite() && alpha.norm_sq() < 1.0).then_some(Self {
            alpha,
            rotation: Complex::ONE,
        })
    }
    pub fn valid(&self) -> bool {
        self.alpha.finite()
            && self.alpha.norm_sq() < 1.0
            && self.rotation.finite()
            && (self.rotation.norm() - 1.0).abs() < 1e-8
    }
    pub fn apply(self, z: Complex) -> Complex {
        self.rotation * (z - self.alpha) / (Complex::ONE - self.alpha.conj() * z)
    }
    pub fn inverse(self) -> Self {
        Self {
            alpha: -self.rotation * self.alpha,
            rotation: self.rotation.conj(),
        }
    }
    fn derivative(self, z: Complex) -> Complex {
        let d = Complex::ONE - self.alpha.conj() * z;
        self.rotation * (1.0 - self.alpha.norm_sq()) / (d * d)
    }
    /// self after inner. Recover parameters from image(0) and derivative(0).
    pub fn compose(self, inner: Self) -> Self {
        let origin = inner.apply(Complex::ZERO);
        let image = self.apply(origin);
        let rotation = (self.derivative(origin) * inner.derivative(Complex::ZERO)).unit();
        Self {
            alpha: (-image / rotation).bounded(1.0 - 1e-12),
            rotation,
        }
    }
    pub fn drag(self, from: Complex, to: Complex) -> Self {
        let a = Self::shift(from.bounded(0.97)).unwrap();
        let b = Self::shift(to.bounded(0.97)).unwrap();
        b.inverse().compose(a).compose(self)
    }
    pub fn center(self, point: Complex) -> Self {
        Self::shift(point.bounded(0.995)).unwrap().compose(self)
    }
    pub fn from_samples(origin: Complex, positive: Complex) -> Self {
        let shift = Self::shift(origin.bounded(1.0 - 1e-12)).unwrap();
        let rotation = shift.apply(positive).unit();
        Self {
            alpha: -origin / rotation,
            rotation,
        }
    }
}

/// det=+1 signed columns mapping the standard (infinity, 0, 1) triangle.
#[derive(Clone, Debug)]
pub struct Frame {
    matrix: [BigInt; 4],
    pub triangle: Triangle,
}
fn basis(t: &Triangle) -> [BigInt; 4] {
    let u = &t.faces()[0];
    let v = &t.faces()[1];
    let w = &t.faces()[2];
    let mut m = [u.p().clone(), v.p().clone(), u.q().clone(), v.q().clone()];
    if u.sum(v).unwrap() != *w {
        m[1] = -m[1].clone();
        m[3] = -m[3].clone();
    }
    if &m[0] * &m[3] - &m[1] * &m[2] < BigInt::from(0) {
        m.swap(0, 1);
        m.swap(2, 3);
    }
    m
}
fn float_matrix(m: &[BigInt; 4]) -> [f64; 4] {
    let bits = m.iter().map(|v| v.bits()).max().unwrap_or(0);
    let shift = bits.saturating_sub(500) as usize;
    m.clone().map(|v| {
        let negative = v.is_negative();
        let x = (v.abs() >> shift).to_f64().unwrap_or(0.0);
        if negative { -x } else { x }
    })
}
fn matrix_disk(m: &[BigInt; 4], w: Complex) -> Complex {
    // Homogeneous Cayley sandwich, so infinity and boundary points need no special division.
    let [a, b, c, d] = float_matrix(m);
    let numerator = Complex::I * (Complex::ONE + w);
    let denominator = Complex::ONE - w;
    let n = numerator * a + denominator * b;
    let q = numerator * c + denominator * d;
    (n - Complex::I * q) / (n + Complex::I * q)
}
impl Frame {
    pub fn new(triangle: Triangle) -> Self {
        Self {
            matrix: basis(&triangle),
            triangle,
        }
    }
    fn local(&self, v: &Primitive) -> Primitive {
        let [a, b, c, d] = &self.matrix;
        Primitive::new(d * v.p() - b * v.q(), -c * v.p() + a * v.q()).unwrap()
    }
    pub fn center(&self, t: &Triangle) -> Complex {
        let local = Triangle::new(t.faces().clone().map(|v| self.local(&v))).unwrap();
        let w = matrix_disk(&basis(&local), root_center());
        Isometry::shift(root_center()).unwrap().apply(w)
    }
    pub fn ideal(&self, v: &Primitive) -> Complex {
        let v = self.local(v);
        let [p, _, q, _] = float_matrix(&[
            v.p().clone(),
            BigInt::from(0),
            v.q().clone(),
            BigInt::from(0),
        ]);
        let w = Complex::new(p, -q) / Complex::new(p, q);
        Isometry::shift(root_center()).unwrap().apply(w).unit()
    }
    /// Isometry carrying new-frame coordinates into this frame.
    pub fn transition(&self, new: &Self) -> Isometry {
        let [a, b, c, d] = &self.matrix;
        let [e, f, g, h] = &new.matrix;
        let relative = [d * e - b * g, d * f - b * h, -c * e + a * g, -c * f + a * h];
        let shift = Isometry::shift(root_center()).unwrap();
        let map = |z| shift.apply(matrix_disk(&relative, shift.inverse().apply(z)));
        Isometry::from_samples(map(Complex::ZERO), map(Complex::new(0.2, 0.0)))
    }
}

/// Points on the actual circle orthogonal to the unit boundary (a diameter if collinear).
pub fn geodesic(a: Complex, b: Complex, steps: usize) -> Vec<Complex> {
    let shift = Isometry::shift(a.bounded(1.0 - 1e-10)).unwrap();
    let end = shift.apply(b);
    let inverse = shift.inverse();
    (0..=steps.max(1))
        .map(|i| {
            inverse
                .apply(end * (i as f64 / steps.max(1) as f64))
                .bounded(1.0)
        })
        .collect()
}
pub fn midpoint(a: Complex, b: Complex) -> Complex {
    let shift = Isometry::shift(a.bounded(1.0 - 1e-10)).unwrap();
    let z = shift.apply(b);
    let r = z.norm().min(1.0 - 1e-12);
    if r < 1e-14 {
        return a;
    }
    shift.inverse().apply(z * (r.atanh() * 0.5).tanh() / r)
}
pub fn distance(a: Complex, b: Complex) -> f64 {
    2.0 * Isometry::shift(a)
        .unwrap()
        .apply(b)
        .norm()
        .min(1.0 - 1e-15)
        .atanh()
}
/// Interior arc approaching an ideal endpoint, excluding the numerical singularity at |z|=1.
pub fn ray(a: Complex, ideal: Complex, steps: usize) -> Vec<Complex> {
    geodesic(a, ideal.bounded(1.0 - 1e-8), steps)
}
/// Complete side between two ideal vertices. Sample from an interior point to
/// avoid applying a disk automorphism centered on the unit boundary.
pub fn ideal_geodesic(a: Complex, b: Complex, steps: usize) -> Vec<Complex> {
    let sum = a + b;
    let closest = sum / (2.0 + (4.0 - sum.norm_sq()).max(0.0).sqrt());
    let mut path = geodesic(closest, a, steps);
    path.reverse();
    path.extend(geodesic(closest, b, steps).into_iter().skip(1));
    path
}
pub fn polygon_contains(poly: &[Complex], p: Complex) -> bool {
    let mut inside = false;
    for i in 0..poly.len() {
        let a = poly[i];
        let b = poly[(i + 1) % poly.len()];
        if (a.im > p.im) != (b.im > p.im)
            && p.re < (b.re - a.re) * (p.im - a.im) / (b.im - a.im) + a.re
        {
            inside = !inside;
        }
    }
    inside
}
