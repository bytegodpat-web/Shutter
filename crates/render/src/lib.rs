//! Evalúa una composición en un instante y la dibuja.
//!
//! El trabajo se divide en dos pasos independientes:
//!
//! 1. [`evaluate`] resuelve la composición en un instante (jerarquía,
//!    animación, opacidad…) y produce una [`Scene`]: una lista plana de
//!    elementos listos para dibujar. No depende de ningún backend.
//! 2. Un backend dibuja la escena. Hoy existe [`raster::render`], un
//!    rasterizador por CPU de referencia; el backend de GPU (`wgpu`) usará la
//!    misma escena.

pub mod raster;
mod scene;

pub use raster::{render, supports_blend, Frame};
pub use scene::{evaluate, Affine, Item, Primitive, Scene};

use shutter_core::Composition;

/// Atajo: evalúa `comp` en `time` y la dibuja por CPU.
pub fn render_at(comp: &Composition, time: f64) -> Option<Frame> {
    render(&evaluate(comp, time))
}
