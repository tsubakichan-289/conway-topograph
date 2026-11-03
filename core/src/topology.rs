use crate::algebra::{MathError, Primitive};

/// One Farey ideal triangle / dual vertex. Its THREE FACES are distinct lax vectors.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Triangle {
    faces: [Primitive; 3],
}
impl Triangle {
    pub fn new(mut faces: [Primitive; 3]) -> Result<Self, MathError> {
        faces.sort();
        if !(faces[0].adjacent(&faces[1])
            && faces[1].adjacent(&faces[2])
            && faces[2].adjacent(&faces[0]))
        {
            return Err(MathError::NotUnimodular);
        }
        Ok(Self { faces })
    }
    pub fn root() -> Self {
        Self::new([
            Primitive::small(1, 0),
            Primitive::small(0, 1),
            Primitive::small(1, 1),
        ])
        .unwrap()
    }
    pub fn faces(&self) -> &[Primitive; 3] {
        &self.faces
    }
    /// Edge opposite faces[opposite]; carries the OTHER TWO faces.
    pub fn edge_faces(&self, opposite: usize) -> [Primitive; 2] {
        [
            self.faces[(opposite + 1) % 3].clone(),
            self.faces[(opposite + 2) % 3].clone(),
        ]
    }
    pub fn neighbor(&self, opposite: usize) -> Self {
        let [u, v] = self.edge_faces(opposite);
        let sum = u.sum(&v).expect("unimodular sum is primitive");
        let difference = u
            .difference(&v)
            .expect("unimodular difference is primitive");
        let other = if sum == self.faces[opposite] {
            difference
        } else {
            sum
        };
        Self::new([u, v, other]).expect("Farey flip preserves adjacency")
    }
    pub fn neighbors(&self) -> [Self; 3] {
        std::array::from_fn(|i| self.neighbor(i))
    }
}
