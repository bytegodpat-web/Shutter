//! Keyframes, interpolación y curvas de easing.
//!
//! Todo valor animable del editor (posición, volumen, parámetros de efectos…)
//! es un [`Property<T>`]: un valor base más una lista ordenada de [`Keyframe`].

mod easing;
mod lerp;
mod property;

pub use easing::Easing;
pub use lerp::Lerp;
pub use property::{Keyframe, Property};
