use num_bigint::BigInt;
use num_traits::{One, Signed, Zero};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MathError {
    #[error("The vector must be primitive: gcd(p,q) = 1 (and nonzero).")]
    NotPrimitive,
    #[error("Adjacent faces must have |det(u,v)| = 1.")]
    NotUnimodular,
    #[error("The three faces are linearly dependent; coefficients are not uniquely determined.")]
    Dependent,
    #[error("These labels determine rational, but not integer, coefficients.")]
    NonIntegral,
    #[error("Invalid integer: {0}")]
    InvalidInteger(String),
}

pub fn gcd(mut a: BigInt, mut b: BigInt) -> BigInt {
    a = a.abs();
    b = b.abs();
    while !b.is_zero() {
        (a, b) = (b.clone(), a % b);
    }
    a
}

/// Canonical lax vector: q > 0, or q = 0 and p > 0.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "[BigInt; 2]", into = "[BigInt; 2]")]
pub struct Primitive {
    p: BigInt,
    q: BigInt,
}
impl TryFrom<[BigInt; 2]> for Primitive {
    type Error = MathError;
    fn try_from([p, q]: [BigInt; 2]) -> Result<Self, Self::Error> {
        Self::new(p, q)
    }
}
impl From<Primitive> for [BigInt; 2] {
    fn from(v: Primitive) -> Self {
        [v.p, v.q]
    }
}
impl std::fmt::Display for Primitive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}, {}]", self.p, self.q)
    }
}
impl Primitive {
    pub fn new(mut p: BigInt, mut q: BigInt) -> Result<Self, MathError> {
        if gcd(p.clone(), q.clone()) != BigInt::one() {
            return Err(MathError::NotPrimitive);
        }
        if q.is_negative() || (q.is_zero() && p.is_negative()) {
            p = -p;
            q = -q;
        }
        Ok(Self { p, q })
    }
    pub fn small(p: i64, q: i64) -> Self {
        Self::new(p.into(), q.into()).expect("primitive constant")
    }
    pub fn p(&self) -> &BigInt {
        &self.p
    }
    pub fn q(&self) -> &BigInt {
        &self.q
    }
    pub fn det(&self, other: &Self) -> BigInt {
        &self.p * &other.q - &self.q * &other.p
    }
    pub fn adjacent(&self, other: &Self) -> bool {
        self.det(other).abs().is_one()
    }
    pub fn sum(&self, other: &Self) -> Result<Self, MathError> {
        Self::new(&self.p + &other.p, &self.q + &other.q)
    }
    pub fn difference(&self, other: &Self) -> Result<Self, MathError> {
        Self::new(&self.p - &other.p, &self.q - &other.q)
    }
    pub fn row(&self) -> [BigInt; 3] {
        [&self.p * &self.p, &self.p * &self.q, &self.q * &self.q]
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Form {
    pub a: BigInt,
    pub b: BigInt,
    pub c: BigInt,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormKind {
    PositiveDefinite,
    NegativeDefinite,
    Indefinite,
    Degenerate,
}
impl Form {
    pub fn small(a: i64, b: i64, c: i64) -> Self {
        Self {
            a: a.into(),
            b: b.into(),
            c: c.into(),
        }
    }
    pub fn evaluate(&self, v: &Primitive) -> BigInt {
        self.evaluate_xy(v.p(), v.q())
    }
    pub fn evaluate_xy(&self, x: &BigInt, y: &BigInt) -> BigInt {
        &self.a * x * x + &self.b * x * y + &self.c * y * y
    }
    pub fn discriminant(&self) -> BigInt {
        &self.b * &self.b - BigInt::from(4) * &self.a * &self.c
    }
    pub fn kind(&self) -> FormKind {
        let d = self.discriminant();
        if d.is_positive() {
            FormKind::Indefinite
        } else if d.is_zero() {
            FormKind::Degenerate
        } else if self.a.is_positive() {
            FormKind::PositiveDefinite
        } else {
            FormKind::NegativeDefinite
        }
    }
    pub fn square_discriminant(&self) -> bool {
        let d = self.discriminant();
        if d.is_negative() {
            return false;
        }
        let r = d.sqrt();
        &r * &r == d
    }
    pub fn from_initial(a: BigInt, c: BigInt, s: BigInt) -> Self {
        let b = s - &a - &c;
        Self { a, b, c }
    }
    /// Cramer's rule with exact determinants and exact divisibility checks.
    pub fn restore(labels: &[(Primitive, BigInt); 3]) -> Result<Self, MathError> {
        let matrix = labels.clone().map(|(v, _)| v.row());
        let denominator = determinant(&matrix);
        if denominator.is_zero() {
            return Err(MathError::Dependent);
        }
        let mut coefficients = Vec::with_capacity(3);
        for column in 0..3 {
            let mut replaced = matrix.clone();
            for row in 0..3 {
                replaced[row][column] = labels[row].1.clone();
            }
            let numerator = determinant(&replaced);
            if (&numerator % &denominator) != BigInt::zero() {
                return Err(MathError::NonIntegral);
            }
            coefficients.push(numerator / &denominator);
        }
        Ok(Self {
            a: coefficients[0].clone(),
            b: coefficients[1].clone(),
            c: coefficients[2].clone(),
        })
    }
    pub fn propagated(&self, u: &Primitive, v: &Primitive, known: &BigInt) -> BigInt {
        BigInt::from(2) * (self.evaluate(u) + self.evaluate(v)) - known
    }
}
fn determinant(m: &[[BigInt; 3]; 3]) -> BigInt {
    &m[0][0] * (&m[1][1] * &m[2][2] - &m[1][2] * &m[2][1])
        - &m[0][1] * (&m[1][0] * &m[2][2] - &m[1][2] * &m[2][0])
        + &m[0][2] * (&m[1][0] * &m[2][1] - &m[1][1] * &m[2][0])
}
pub fn parse_integer(s: &str) -> Result<BigInt, MathError> {
    s.trim()
        .parse()
        .map_err(|_| MathError::InvalidInteger(s.to_owned()))
}
