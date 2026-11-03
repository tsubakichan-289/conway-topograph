use serde::{Deserialize, Serialize};
use thiserror::Error;
use topograph_core::{
    algebra::{Form, Primitive, parse_integer},
    editor::{Editor, FixedFace},
    geometry::{Frame, Isometry},
    topology::Triangle,
};
#[derive(Debug, Error)]
pub enum StorageError {
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Math(#[from] topograph_core::algebra::MathError),
    #[error("Invalid saved settings: {0}")]
    Invalid(&'static str),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Saved {
    pub version: u32,
    pub coefficients: [String; 3],
    pub fixed: Vec<SavedPin>,
    pub anchor: [[String; 2]; 3],
    pub view: Isometry,
    pub labels: bool,
    pub rivers: bool,
    pub wells: bool,
    pub lakes: bool,
    pub zoom: f32,
    pub density: f64,
    pub depth: usize,
    #[serde(default)]
    pub palette: crate::render::Palette,
    #[serde(default)]
    pub color_mode: crate::render::ColorMode,
    #[serde(default)]
    pub graph_mode: crate::render::GraphMode,
    #[serde(default = "crate::render::default_value_scale")]
    pub value_scale: u32,
}
#[derive(Serialize, Deserialize)]
pub struct SavedPin {
    pub vector: [String; 2],
    pub value: String,
}
fn vector(v: &Primitive) -> [String; 2] {
    [v.p().to_string(), v.q().to_string()]
}
fn parse_vector(v: &[String; 2]) -> Result<Primitive, StorageError> {
    Ok(Primitive::new(
        parse_integer(&v[0])?,
        parse_integer(&v[1])?,
    )?)
}
impl Saved {
    pub fn capture(app: &crate::app::App) -> Self {
        let f = &app.editor.form;
        Self {
            version: 1,
            coefficients: [f.a.to_string(), f.b.to_string(), f.c.to_string()],
            fixed: app
                .editor
                .fixed
                .iter()
                .map(|p| SavedPin {
                    vector: vector(&p.face),
                    value: p.value.to_string(),
                })
                .collect(),
            anchor: app.frame.triangle.faces().clone().map(|v| vector(&v)),
            view: app.view,
            labels: app.labels,
            rivers: app.rivers,
            wells: app.wells,
            lakes: app.lakes,
            zoom: app.zoom,
            density: app.density,
            depth: app.depth,
            palette: app.palette,
            color_mode: app.color_mode,
            graph_mode: app.graph_mode,
            value_scale: app.value_scale,
        }
    }
    pub fn validate(&self) -> Result<(Editor, Frame), StorageError> {
        if self.version != 1 {
            return Err(StorageError::Invalid("unsupported version"));
        }
        if self.fixed.len() > 3 {
            return Err(StorageError::Invalid("more than three fixed faces"));
        }
        if !self.view.valid()
            || !self.zoom.is_finite()
            || !(0.35..=1.0).contains(&self.zoom)
            || !self.density.is_finite()
            || !(2.0..=18.0).contains(&self.density)
            || !(8..=64).contains(&self.depth)
            || !(1..=1_000_000).contains(&self.value_scale)
        {
            return Err(StorageError::Invalid("invalid view or limits"));
        }
        let form = Form {
            a: parse_integer(&self.coefficients[0])?,
            b: parse_integer(&self.coefficients[1])?,
            c: parse_integer(&self.coefficients[2])?,
        };
        let mut fixed = Vec::<FixedFace>::new();
        for p in &self.fixed {
            let face = parse_vector(&p.vector)?;
            let value = parse_integer(&p.value)?;
            if form.evaluate(&face) != value || fixed.iter().any(|p| p.face == face) {
                return Err(StorageError::Invalid(
                    "fixed labels contradict coefficients or duplicate a face",
                ));
            }
            fixed.push(FixedFace { face, value });
        }
        let faces = [
            parse_vector(&self.anchor[0])?,
            parse_vector(&self.anchor[1])?,
            parse_vector(&self.anchor[2])?,
        ];
        Ok((Editor { form, fixed }, Frame::new(Triangle::new(faces)?)))
    }
    pub fn to_json(&self) -> Result<String, StorageError> {
        Ok(serde_json::to_string_pretty(self)?)
    }
    pub fn from_json(json: &str) -> Result<Self, StorageError> {
        let s: Self = serde_json::from_str(json)?;
        s.validate()?;
        Ok(s)
    }
}
