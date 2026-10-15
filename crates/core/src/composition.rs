use crate::{Layer, LayerId, LayerKind};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt};

/// Relación de aspecto. `Custom` se interpreta como `width:height`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AspectRatio {
    R16x9,
    R9x16,
    R4x5,
    R1x1,
    R4x3,
    Custom { width: u32, height: u32 },
}

impl AspectRatio {
    fn parts(self) -> (u32, u32) {
        match self {
            AspectRatio::R16x9 => (16, 9),
            AspectRatio::R9x16 => (9, 16),
            AspectRatio::R4x5 => (4, 5),
            AspectRatio::R1x1 => (1, 1),
            AspectRatio::R4x3 => (4, 3),
            AspectRatio::Custom { width, height } => (width.max(1), height.max(1)),
        }
    }
}

/// Resolución nominal: es el lado **corto** de la composición
/// (1080p en 9:16 da 1080×1920).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Resolution {
    P180,
    P270,
    P360,
    P540,
    P720,
    P1080,
    P1440,
    P2160,
}

impl Resolution {
    pub const ALL: [Resolution; 8] = [
        Resolution::P180,
        Resolution::P270,
        Resolution::P360,
        Resolution::P540,
        Resolution::P720,
        Resolution::P1080,
        Resolution::P1440,
        Resolution::P2160,
    ];

    pub fn short_side(self) -> u32 {
        match self {
            Resolution::P180 => 180,
            Resolution::P270 => 270,
            Resolution::P360 => 360,
            Resolution::P540 => 540,
            Resolution::P720 => 720,
            Resolution::P1080 => 1080,
            Resolution::P1440 => 1440,
            Resolution::P2160 => 2160,
        }
    }

    /// Ancho y alto resultantes (redondeados a un número par).
    pub fn dimensions(self, aspect: AspectRatio) -> (u32, u32) {
        let short = self.short_side() as f64;
        let (w, h) = aspect.parts();
        let even = |v: f64| ((v / 2.0).round() * 2.0) as u32;
        if w >= h {
            (even(short * w as f64 / h as f64), even(short))
        } else {
            (even(short), even(short * h as f64 / w as f64))
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Background {
    #[default]
    Black,
    LightGray,
    White,
    Green,
    Blue,
    Transparent,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HierarchyError {
    LayerNotFound(LayerId),
    NotAGroup(LayerId),
    WouldCreateCycle,
}

impl fmt::Display for HierarchyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HierarchyError::LayerNotFound(id) => write!(f, "la capa {id} no existe"),
            HierarchyError::NotAGroup(id) => write!(f, "la capa {id} no es un grupo"),
            HierarchyError::WouldCreateCycle => write!(f, "la jerarquía tendría un ciclo"),
        }
    }
}

impl std::error::Error for HierarchyError {}

/// Una composición: lienzo, tiempo y la lista de capas (de abajo hacia arriba).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Composition {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub background: Background,
    /// Duración en segundos.
    pub duration: f64,
    layers: Vec<Layer>,
    next_id: LayerId,
}

impl Composition {
    pub fn new(aspect: AspectRatio, resolution: Resolution, fps: u32, background: Background) -> Self {
        let (width, height) = resolution.dimensions(aspect);
        Self { width, height, fps: fps.max(1), background, duration: 5.0, layers: Vec::new(), next_id: 1 }
    }

    /// Capas en orden de apilado (la primera queda debajo).
    pub fn layers(&self) -> &[Layer] {
        &self.layers
    }

    pub fn layer(&self, id: LayerId) -> Option<&Layer> {
        self.layers.iter().find(|l| l.id == id)
    }

    pub fn layer_mut(&mut self, id: LayerId) -> Option<&mut Layer> {
        self.layers.iter_mut().find(|l| l.id == id)
    }

    /// Añade una capa centrada en el lienzo (la cámara se sitúa a la distancia
    /// que encuadra el ancho completo con su campo de visión).
    pub fn add_layer(&mut self, name: impl Into<String>, mut kind: LayerKind) -> LayerId {
        let id = self.next_id;
        self.next_id += 1;

        let (cx, cy) = (self.width as f32 / 2.0, self.height as f32 / 2.0);
        let mut position = [cx, cy, 0.0];

        if let LayerKind::Camera(cam) = &mut kind {
            let fov = *cam.fov.base();
            let distance = cx / (fov.to_radians() / 2.0).tan();
            position[2] = -distance;
            cam.focus_distance.set_base(distance);
            cam.fog_near.set_base(distance);
            cam.fog_far.set_base(distance * 2.0);
        }

        let mut layer = Layer::new(id, name.into(), kind, position, self.duration);
        // Las formas nuevas ocupan un cuarto del lado corto del lienzo.
        let side = self.width.min(self.height) as f32 / 4.0;
        layer.size.set_base([side, side]);
        self.layers.push(layer);
        id
    }

    /// Siguiente nombre libre con la forma `"{base} {n}"`.
    pub fn unique_name(&self, base: &str) -> String {
        (1..)
            .map(|n| format!("{base} {n}"))
            .find(|candidate| !self.layers.iter().any(|l| &l.name == candidate))
            .expect("el iterador es infinito")
    }

    /// Asigna (o quita) el padre de transformación de una capa.
    pub fn set_parent(&mut self, id: LayerId, parent: Option<LayerId>) -> Result<(), HierarchyError> {
        self.require(id)?;
        if let Some(p) = parent {
            self.require(p)?;
            if self.would_cycle(id, p, |l| l.parent) {
                return Err(HierarchyError::WouldCreateCycle);
            }
        }
        self.layer_mut(id).expect("comprobado arriba").parent = parent;
        Ok(())
    }

    /// Mete una capa en un grupo (o la saca con `None`).
    pub fn set_container(&mut self, id: LayerId, group: Option<LayerId>) -> Result<(), HierarchyError> {
        self.require(id)?;
        if let Some(g) = group {
            let target = self.layer(g).ok_or(HierarchyError::LayerNotFound(g))?;
            if !matches!(target.kind, LayerKind::Group) {
                return Err(HierarchyError::NotAGroup(g));
            }
            if self.would_cycle(id, g, |l| l.container) {
                return Err(HierarchyError::WouldCreateCycle);
            }
        }
        self.layer_mut(id).expect("comprobado arriba").container = group;
        Ok(())
    }

    /// Hijas directas de `id` en la jerarquía padre-hijo.
    pub fn children(&self, id: LayerId) -> Vec<LayerId> {
        self.layers.iter().filter(|l| l.parent == Some(id)).map(|l| l.id).collect()
    }

    /// Capas contenidas directamente en el grupo `id`.
    pub fn members(&self, id: LayerId) -> Vec<LayerId> {
        self.layers.iter().filter(|l| l.container == Some(id)).map(|l| l.id).collect()
    }

    /// Elimina una capa. Si es un grupo elimina también su contenido; las
    /// capas que la tenían como padre quedan sin padre.
    /// Devuelve las capas eliminadas (primero la pedida).
    pub fn remove_layer(&mut self, id: LayerId) -> Vec<Layer> {
        if self.layer(id).is_none() {
            return Vec::new();
        }
        let doomed = self.with_members(id);

        let mut removed = Vec::new();
        let mut kept = Vec::new();
        for layer in self.layers.drain(..) {
            if doomed.contains(&layer.id) {
                removed.push(layer);
            } else {
                kept.push(layer);
            }
        }
        for layer in &mut kept {
            if layer.parent.is_some_and(|p| doomed.contains(&p)) {
                layer.parent = None;
            }
        }
        self.layers = kept;

        removed.sort_by_key(|l| l.id != id); // la pedida primero; sort_by_key es estable
        removed
    }

    /// Duplica una capa (y, si es un grupo, su contenido). Devuelve el id de la copia.
    /// Las copias se colocan justo encima del original.
    pub fn duplicate_layer(&mut self, id: LayerId) -> Option<LayerId> {
        let set = self.with_members(id);
        if set.is_empty() {
            return None;
        }

        // Originales en orden de apilado.
        let originals: Vec<Layer> = self.layers.iter().filter(|l| set.contains(&l.id)).cloned().collect();
        let insert_at = self.layers.iter().rposition(|l| set.contains(&l.id)).expect("existe") + 1;

        let mut map: HashMap<LayerId, LayerId> = HashMap::new();
        for l in &originals {
            map.insert(l.id, self.next_id);
            self.next_id += 1;
        }

        let copy_name = self.copy_name(&self.layer(id).expect("existe").name);
        let clones: Vec<Layer> = originals
            .into_iter()
            .map(|mut l| {
                let is_root = l.id == id;
                l.id = map[&l.id];
                l.parent = l.parent.map(|p| map.get(&p).copied().unwrap_or(p));
                l.container = l.container.map(|c| map.get(&c).copied().unwrap_or(c));
                if is_root {
                    l.name = copy_name.clone();
                }
                l
            })
            .collect();

        self.layers.splice(insert_at..insert_at, clones);
        Some(map[&id])
    }

    /// Mueve una capa a otra posición del apilado. Devuelve `false` si no existe.
    pub fn move_layer(&mut self, id: LayerId, new_index: usize) -> bool {
        let Some(from) = self.layers.iter().position(|l| l.id == id) else { return false };
        let layer = self.layers.remove(from);
        let to = new_index.min(self.layers.len());
        self.layers.insert(to, layer);
        true
    }

    fn require(&self, id: LayerId) -> Result<(), HierarchyError> {
        self.layer(id).map(|_| ()).ok_or(HierarchyError::LayerNotFound(id))
    }

    /// ¿Seguir la cadena `next` desde `start` llega a `id`?
    fn would_cycle(&self, id: LayerId, start: LayerId, next: impl Fn(&Layer) -> Option<LayerId>) -> bool {
        let mut current = Some(start);
        for _ in 0..=self.layers.len() {
            let Some(c) = current else { return false };
            if c == id {
                return true;
            }
            current = self.layer(c).and_then(&next);
        }
        true // cadena más larga que el número de capas: ya había un ciclo
    }

    /// `id` más todas las capas que contiene (recursivamente).
    fn with_members(&self, id: LayerId) -> Vec<LayerId> {
        let mut set = vec![id];
        loop {
            let before = set.len();
            for l in &self.layers {
                if l.container.is_some_and(|c| set.contains(&c)) && !set.contains(&l.id) {
                    set.push(l.id);
                }
            }
            if set.len() == before {
                return set;
            }
        }
    }

    fn copy_name(&self, name: &str) -> String {
        let base = format!("{name} Copy");
        if !self.layers.iter().any(|l| l.name == base) {
            return base;
        }
        (2..)
            .map(|n| format!("{base} {n}"))
            .find(|candidate| !self.layers.iter().any(|l| &l.name == candidate))
            .expect("el iterador es infinito")
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub composition: Composition,
}

impl Project {
    pub fn new(name: impl Into<String>, composition: Composition) -> Self {
        Self { name: name.into(), composition }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CameraOptions, ShapeKind};
    use shutter_anim::Easing;

    fn comp() -> Composition {
        Composition::new(AspectRatio::R1x1, Resolution::P1080, 30, Background::Black)
    }

    fn shape() -> LayerKind {
        LayerKind::Shape(ShapeKind::Rectangle)
    }

    #[test]
    fn dimensions_follow_the_short_side() {
        let d = |r: Resolution, a| r.dimensions(a);
        assert_eq!(d(Resolution::P1080, AspectRatio::R16x9), (1920, 1080));
        assert_eq!(d(Resolution::P1080, AspectRatio::R9x16), (1080, 1920));
        assert_eq!(d(Resolution::P1080, AspectRatio::R4x5), (1080, 1350));
        assert_eq!(d(Resolution::P1080, AspectRatio::R1x1), (1080, 1080));
        assert_eq!(d(Resolution::P1080, AspectRatio::R4x3), (1440, 1080));
        assert_eq!(d(Resolution::P2160, AspectRatio::R16x9), (3840, 2160));
        assert_eq!(d(Resolution::P180, AspectRatio::R16x9), (320, 180));
    }

    #[test]
    fn dimensions_are_always_even() {
        for r in Resolution::ALL {
            let (w, h) = r.dimensions(AspectRatio::Custom { width: 7, height: 5 });
            assert_eq!(w % 2, 0);
            assert_eq!(h % 2, 0);
        }
    }

    #[test]
    fn new_layers_are_centered_and_get_unique_ids() {
        let mut c = comp();
        let a = c.add_layer("A", shape());
        let b = c.add_layer("B", shape());
        assert_ne!(a, b);
        assert_eq!(c.layer(a).unwrap().transform.position.value_at(0.0), [540.0, 540.0, 0.0]);
    }

    #[test]
    fn new_layers_are_a_quarter_of_the_short_side() {
        let mut c = comp();
        let id = c.add_layer("A", shape());
        assert_eq!(c.layer(id).unwrap().size.value_at(0.0), [270.0, 270.0]);
    }

    #[test]
    fn camera_is_placed_to_frame_the_full_width() {
        let mut c = Composition::new(AspectRatio::R4x3, Resolution::P1080, 30, Background::Black);
        let id = c.add_layer("Cam", LayerKind::Camera(CameraOptions::default()));
        let z = c.layer(id).unwrap().transform.position.value_at(0.0)[2];
        // 720 / tan(30°) ≈ 1247.1
        assert!((z + 1247.1).abs() < 0.2, "z = {z}");
    }

    #[test]
    fn unique_name_skips_taken_names() {
        let mut c = comp();
        c.add_layer("Forma 1", shape());
        c.add_layer("Forma 2", shape());
        assert_eq!(c.unique_name("Forma"), "Forma 3");
        assert_eq!(c.unique_name("Nulo"), "Nulo 1");
    }

    #[test]
    fn parenting_works_and_rejects_cycles() {
        let mut c = comp();
        let a = c.add_layer("A", shape());
        let b = c.add_layer("B", shape());
        let n = c.add_layer("N", LayerKind::Null);

        c.set_parent(a, Some(n)).unwrap();
        c.set_parent(b, Some(a)).unwrap();
        assert_eq!(c.children(n), vec![a]);

        assert_eq!(c.set_parent(a, Some(a)), Err(HierarchyError::WouldCreateCycle));
        assert_eq!(c.set_parent(n, Some(b)), Err(HierarchyError::WouldCreateCycle));
        assert_eq!(c.set_parent(a, Some(999)), Err(HierarchyError::LayerNotFound(999)));

        c.set_parent(a, None).unwrap();
        assert!(c.children(n).is_empty());
    }

    #[test]
    fn containers_must_be_groups() {
        let mut c = comp();
        let g = c.add_layer("G", LayerKind::Group);
        let a = c.add_layer("A", shape());
        let b = c.add_layer("B", shape());

        c.set_container(a, Some(g)).unwrap();
        assert_eq!(c.members(g), vec![a]);
        assert_eq!(c.set_container(a, Some(b)), Err(HierarchyError::NotAGroup(b)));
        assert_eq!(c.set_container(g, Some(g)), Err(HierarchyError::WouldCreateCycle));

        let inner = c.add_layer("Inner", LayerKind::Group);
        c.set_container(inner, Some(g)).unwrap();
        assert_eq!(c.set_container(g, Some(inner)), Err(HierarchyError::WouldCreateCycle));
    }

    #[test]
    fn removing_a_parent_orphans_its_children() {
        let mut c = comp();
        let n = c.add_layer("N", LayerKind::Null);
        let a = c.add_layer("A", shape());
        c.set_parent(a, Some(n)).unwrap();

        let removed = c.remove_layer(n);
        assert_eq!(removed.len(), 1);
        assert_eq!(c.layer(a).unwrap().parent, None);
    }

    #[test]
    fn removing_a_group_removes_its_contents() {
        let mut c = comp();
        let keep = c.add_layer("Keep", shape());
        let g = c.add_layer("G", LayerKind::Group);
        let a = c.add_layer("A", shape());
        c.set_container(a, Some(g)).unwrap();

        let removed = c.remove_layer(g);
        assert_eq!(removed.iter().map(|l| l.id).collect::<Vec<_>>(), vec![g, a]);
        assert_eq!(c.layers().len(), 1);
        assert!(c.layer(keep).is_some());
        assert!(c.remove_layer(g).is_empty());
    }

    #[test]
    fn duplicate_names_follow_copy_copy_2() {
        let mut c = comp();
        let a = c.add_layer("Rect", shape());
        let c1 = c.duplicate_layer(a).unwrap();
        let c2 = c.duplicate_layer(a).unwrap();
        assert_eq!(c.layer(c1).unwrap().name, "Rect Copy");
        assert_eq!(c.layer(c2).unwrap().name, "Rect Copy 2");
    }

    #[test]
    fn duplicate_is_placed_above_the_original() {
        let mut c = comp();
        let a = c.add_layer("A", shape());
        let b = c.add_layer("B", shape());
        let a2 = c.duplicate_layer(a).unwrap();
        let order: Vec<_> = c.layers().iter().map(|l| l.id).collect();
        assert_eq!(order, vec![a, a2, b]);
    }

    #[test]
    fn duplicating_a_group_copies_its_members_and_remaps_links() {
        let mut c = comp();
        let g = c.add_layer("G", LayerKind::Group);
        let a = c.add_layer("A", shape());
        let b = c.add_layer("B", shape());
        c.set_container(a, Some(g)).unwrap();
        c.set_container(b, Some(g)).unwrap();
        c.set_parent(b, Some(a)).unwrap();

        let g2 = c.duplicate_layer(g).unwrap();
        assert_eq!(c.layers().len(), 6);

        let new_members = c.members(g2);
        assert_eq!(new_members.len(), 2);
        // La copia de B tiene como padre la copia de A, no el original.
        let b2 = c.layer(new_members[1]).unwrap();
        assert_eq!(b2.parent, Some(new_members[0]));
        // Los originales no cambian.
        assert_eq!(c.layer(b).unwrap().parent, Some(a));
        assert_eq!(c.members(g), vec![a, b]);
    }

    #[test]
    fn move_layer_reorders() {
        let mut c = comp();
        let a = c.add_layer("A", shape());
        let b = c.add_layer("B", shape());
        let d = c.add_layer("D", shape());
        assert!(c.move_layer(d, 0));
        assert_eq!(c.layers().iter().map(|l| l.id).collect::<Vec<_>>(), vec![d, a, b]);
        assert!(c.move_layer(d, 99));
        assert_eq!(c.layers().iter().map(|l| l.id).collect::<Vec<_>>(), vec![a, b, d]);
        assert!(!c.move_layer(999, 0));
    }

    #[test]
    fn project_survives_a_json_roundtrip_with_animation() {
        let mut c = comp();
        let id = c.add_layer("Rect", shape());
        {
            let l = c.layer_mut(id).unwrap();
            l.transform.position.set_key_with_easing(0.0, [0.0, 0.0, 0.0], Some(Easing::ease_in_out()));
            l.transform.position.set_key(1.0, [100.0, 0.0, 0.0]);
            l.opacity.edit_at(0.0, 0.5);
        }
        let audio = LayerKind::Audio { path: "a.wav".into(), volume: shutter_anim::Property::new(100.0) };
        let g = c.add_layer("Audio", audio);
        c.set_parent(g, Some(id)).unwrap();

        let project = Project::new("Demo", c);
        let json = serde_json::to_string(&project).unwrap();
        let back: Project = serde_json::from_str(&json).unwrap();
        assert_eq!(project, back);
    }
}
