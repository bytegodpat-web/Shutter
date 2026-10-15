use crate::BlendMode;
use serde::{Deserialize, Serialize};
use shutter_anim::Property;

pub type LayerId = u64;

/// Azul por defecto de las formas nuevas (#4792E1).
pub const DEFAULT_FILL: [f32; 4] = [71.0 / 255.0, 146.0 / 255.0, 225.0 / 255.0, 1.0];

/// Relleno de una capa. Los colores son RGBA en `[0, 1]`, sin premultiplicar.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Fill {
    None,
    Solid(Property<[f32; 4]>),
}

impl Default for Fill {
    fn default() -> Self {
        Fill::Solid(Property::new(DEFAULT_FILL))
    }
}

/// Transformación 2D/3D de una capa. Todos los campos son animables.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Transform {
    /// Posición X, Y y Z, en píxeles de la composición.
    pub position: Property<[f32; 3]>,
    /// Rotación en grados.
    pub rotation: Property<f32>,
    /// Escala X e Y (1.0 = 100 %).
    pub scale: Property<[f32; 2]>,
    /// Inclinación X e Y en grados.
    pub skew: Property<[f32; 2]>,
}

impl Transform {
    pub fn at(position: [f32; 3]) -> Self {
        Self {
            position: Property::new(position),
            rotation: Property::new(0.0),
            scale: Property::new([1.0, 1.0]),
            skew: Property::new([0.0, 0.0]),
        }
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::at([0.0, 0.0, 0.0])
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ShapeKind {
    Rectangle,
    Ellipse,
    Polygon { sides: u32 },
    Star { points: u32, inner_ratio: f32 },
}

/// Opciones de la capa de cámara 3D.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CameraOptions {
    /// Campo de visión en grados.
    pub fov: Property<f32>,
    pub blur_enabled: bool,
    pub focus_distance: Property<f32>,
    pub depth_of_field: Property<f32>,
    pub blur_strength: Property<f32>,
    pub fog_enabled: bool,
    pub fog_color: [f32; 4],
    pub fog_near: Property<f32>,
    pub fog_far: Property<f32>,
}

impl Default for CameraOptions {
    fn default() -> Self {
        Self {
            fov: Property::new(60.0),
            blur_enabled: false,
            focus_distance: Property::new(1247.0),
            depth_of_field: Property::new(125.0),
            blur_strength: Property::new(0.5),
            fog_enabled: false,
            fog_color: [1.0, 1.0, 1.0, 1.0],
            fog_near: Property::new(1247.0),
            fog_far: Property::new(2494.0),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum LayerKind {
    Shape(ShapeKind),
    Text {
        content: String,
        font: String,
        size: f32,
    },
    Image {
        path: String,
    },
    Video {
        path: String,
        speed: Property<f32>,
        volume: Property<f32>,
    },
    /// El volumen va de 0 a 100.
    Audio {
        path: String,
        volume: Property<f32>,
    },
    /// Trazos a mano alzada.
    Drawing,
    Camera(CameraOptions),
    /// Contenedor de capas; se renderiza en su propia textura (base de las máscaras).
    Group,
    /// Capa auxiliar sin render, usada como padre de otras.
    Null,
    /// Otra composición reutilizada como capa.
    Nested {
        composition: String,
    },
}

impl LayerKind {
    /// `true` si la capa produce imagen (todas menos audio, nulo y cámara).
    pub fn is_visual(&self) -> bool {
        !matches!(self, LayerKind::Audio { .. } | LayerKind::Null | LayerKind::Camera(_))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Layer {
    pub id: LayerId,
    pub name: String,
    pub kind: LayerKind,
    pub visible: bool,
    /// Capa padre para la transformación (jerarquía padre-hijo).
    pub parent: Option<LayerId>,
    /// Grupo que contiene a esta capa.
    pub container: Option<LayerId>,
    pub transform: Transform,
    /// Ancho y alto base en píxeles, antes de aplicar la escala.
    pub size: Property<[f32; 2]>,
    pub fill: Fill,
    /// De 0.0 a 1.0.
    pub opacity: Property<f32>,
    pub blend: BlendMode,
    /// Segundos en los que la capa entra y sale en el timeline.
    pub start: f64,
    pub end: f64,
}

impl Layer {
    pub(crate) fn new(id: LayerId, name: String, kind: LayerKind, position: [f32; 3], end: f64) -> Self {
        Self {
            id,
            name,
            kind,
            visible: true,
            parent: None,
            container: None,
            transform: Transform::at(position),
            size: Property::new([200.0, 200.0]),
            fill: Fill::default(),
            opacity: Property::new(1.0),
            blend: BlendMode::Normal,
            start: 0.0,
            end,
        }
    }

    /// ¿Está la capa activa en el instante `time`?
    pub fn is_active_at(&self, time: f64) -> bool {
        self.visible && time >= self.start && time < self.end
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transform_defaults() {
        let t = Transform::at([540.0, 540.0, 0.0]);
        assert_eq!(t.position.value_at(0.0), [540.0, 540.0, 0.0]);
        assert_eq!(t.scale.value_at(0.0), [1.0, 1.0]);
    }

    #[test]
    fn camera_defaults_match_the_reference_values() {
        let c = CameraOptions::default();
        assert_eq!(c.fov.value_at(0.0), 60.0);
        assert_eq!(c.fog_far.value_at(0.0), 2.0 * c.fog_near.value_at(0.0));
    }

    #[test]
    fn visual_kinds() {
        assert!(LayerKind::Drawing.is_visual());
        assert!(LayerKind::Group.is_visual());
        assert!(!LayerKind::Null.is_visual());
        assert!(!LayerKind::Camera(CameraOptions::default()).is_visual());
        assert!(!LayerKind::Audio { path: String::new(), volume: Property::new(100.0) }.is_visual());
    }

    #[test]
    fn new_layers_have_a_default_size_and_blue_fill() {
        let l = Layer::new(1, "a".into(), LayerKind::Drawing, [0.0; 3], 5.0);
        assert_eq!(l.size.value_at(0.0), [200.0, 200.0]);
        assert_eq!(l.fill, Fill::Solid(Property::new(DEFAULT_FILL)));
    }

    #[test]
    fn activity_window() {
        let l = Layer::new(1, "a".into(), LayerKind::Drawing, [0.0; 3], 5.0);
        assert!(l.is_active_at(0.0));
        assert!(l.is_active_at(4.99));
        assert!(!l.is_active_at(5.0));
        let mut hidden = l.clone();
        hidden.visible = false;
        assert!(!hidden.is_active_at(1.0));
    }
}
