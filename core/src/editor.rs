use crate::algebra::{Form, MathError, Primitive};
use num_bigint::BigInt;
use thiserror::Error;
#[derive(Clone, Debug)]
pub struct FixedFace {
    pub face: Primitive,
    pub value: BigInt,
}
#[derive(Clone, Debug)]
pub struct Editor {
    pub form: Form,
    pub fixed: Vec<FixedFace>,
}
#[derive(Debug, Error)]
pub enum EditError {
    #[error(transparent)]
    Math(#[from] MathError),
    #[error("Select exactly three fixed faces before editing a label.")]
    NeedThree,
    #[error("This face is not fixed. Add it, or replace one of the three fixed faces.")]
    NotFixed,
    #[error("At most three faces can be fixed. Remove or replace a fixed face first.")]
    TooMany,
}
impl Default for Editor {
    fn default() -> Self {
        let mut e = Self {
            form: Form::small(1, 0, 1),
            fixed: Vec::new(),
        };
        e.initial_faces();
        e
    }
}
impl Editor {
    pub fn initial_faces(&mut self) {
        self.fixed = [
            Primitive::small(1, 0),
            Primitive::small(0, 1),
            Primitive::small(1, 1),
        ]
        .into_iter()
        .map(|face| FixedFace {
            value: self.form.evaluate(&face),
            face,
        })
        .collect();
    }
    pub fn set_form(&mut self, form: Form) {
        self.form = form;
        for p in &mut self.fixed {
            p.value = self.form.evaluate(&p.face);
        }
    }
    pub fn pin(&mut self, face: Primitive) -> Result<(), EditError> {
        if self.fixed.iter().any(|p| p.face == face) {
            return Ok(());
        }
        if self.fixed.len() == 3 {
            return Err(EditError::TooMany);
        }
        self.fixed.push(FixedFace {
            value: self.form.evaluate(&face),
            face,
        });
        Ok(())
    }
    pub fn replace(&mut self, index: usize, face: Primitive) -> Result<(), EditError> {
        if index >= self.fixed.len() {
            return Err(EditError::NotFixed);
        }
        if self
            .fixed
            .iter()
            .enumerate()
            .any(|(i, p)| i != index && p.face == face)
        {
            return Err(MathError::Dependent.into());
        }
        self.fixed[index] = FixedFace {
            value: self.form.evaluate(&face),
            face,
        };
        Ok(())
    }
    pub fn apply_labels(&mut self, values: &[BigInt]) -> Result<(), EditError> {
        if self.fixed.len() != 3 || values.len() != 3 {
            return Err(EditError::NeedThree);
        }
        let constraints = std::array::from_fn(|i| (self.fixed[i].face.clone(), values[i].clone()));
        let form = Form::restore(&constraints)?;
        self.set_form(form);
        Ok(())
    }
    pub fn edit_face(&mut self, face: &Primitive, value: BigInt) -> Result<(), EditError> {
        let index = self
            .fixed
            .iter()
            .position(|p| &p.face == face)
            .ok_or(EditError::NotFixed)?;
        let mut values: Vec<_> = self.fixed.iter().map(|p| p.value.clone()).collect();
        values[index] = value;
        self.apply_labels(&values)
    }
}
