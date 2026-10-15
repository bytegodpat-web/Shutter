//! Modelo del documento: proyecto, composición, capas y propiedades.
//!
//! Este crate no sabe nada de render, interfaz ni E/S de medios. Todo el
//! documento es serializable con `serde`, y todo valor animable es un
//! [`shutter_anim::Property`].

mod blend;
mod composition;
mod layer;

pub use blend::{BlendGroup, BlendMode};
pub use composition::{AspectRatio, Background, Composition, HierarchyError, Project, Resolution};
pub use layer::{CameraOptions, Layer, LayerId, LayerKind, ShapeKind, Transform};
