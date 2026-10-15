//! Evaluación de una composición en un instante: qué capas se dibujan, dónde
//! y con qué opacidad. No depende de ningún backend de dibujo.

use shutter_core::{BlendMode, Composition, Fill, Layer, LayerId, LayerKind, ShapeKind};

/// Transformación afín 2D. Mapea `(x, y)` a `(a·x + c·y + e, b·x + d·y + f)`,
/// el mismo orden de campos que usan Skia y SVG.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Affine {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub e: f32,
    pub f: f32,
}

impl Affine {
    pub const IDENTITY: Affine = Affine { a: 1.0, b: 0.0, c: 0.0, d: 1.0, e: 0.0, f: 0.0 };

    pub fn translate(x: f32, y: f32) -> Self {
        Affine { e: x, f: y, ..Self::IDENTITY }
    }

    pub fn scale(x: f32, y: f32) -> Self {
        Affine { a: x, d: y, ..Self::IDENTITY }
    }

    /// Rotación en grados; con el eje Y hacia abajo, un ángulo positivo gira en sentido horario.
    pub fn rotate(degrees: f32) -> Self {
        let (s, c) = degrees.to_radians().sin_cos();
        Affine { a: c, b: s, c: -s, d: c, e: 0.0, f: 0.0 }
    }

    /// Inclinación horizontal y vertical en grados.
    pub fn skew(x_degrees: f32, y_degrees: f32) -> Self {
        Affine { b: y_degrees.to_radians().tan(), c: x_degrees.to_radians().tan(), ..Self::IDENTITY }
    }

    /// `self ∘ other`: aplica primero `other` y después `self`.
    pub fn then_apply_to(self, other: Affine) -> Affine {
        let (p, q) = (self, other);
        Affine {
            a: p.a * q.a + p.c * q.b,
            b: p.b * q.a + p.d * q.b,
            c: p.a * q.c + p.c * q.d,
            d: p.b * q.c + p.d * q.d,
            e: p.a * q.e + p.c * q.f + p.e,
            f: p.b * q.e + p.d * q.f + p.f,
        }
    }

    pub fn apply(&self, x: f32, y: f32) -> (f32, f32) {
        (self.a * x + self.c * y + self.e, self.b * x + self.d * y + self.f)
    }
}

/// Qué hay que dibujar en un elemento.
#[derive(Clone, Debug, PartialEq)]
pub enum Primitive {
    /// Forma centrada en el origen del elemento, con ancho y alto en píxeles.
    Shape { kind: ShapeKind, width: f32, height: f32 },
}

/// Una capa ya resuelta para un instante concreto.
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub layer: LayerId,
    pub primitive: Primitive,
    /// Del espacio de la capa al del lienzo, con la cadena de padres incluida.
    pub transform: Affine,
    /// Opacidad propia multiplicada por la de los grupos que la contienen.
    pub opacity: f32,
    /// Color RGBA sin premultiplicar; `None` si no tiene relleno.
    pub fill: Option<[f32; 4]>,
    pub blend: BlendMode,
}

/// Lo que se dibuja en un instante, de abajo hacia arriba.
#[derive(Clone, Debug, PartialEq)]
pub struct Scene {
    pub width: u32,
    pub height: u32,
    pub background: shutter_core::Background,
    pub items: Vec<Item>,
}

/// Evalúa `comp` en el instante `time` (segundos).
///
/// Por ahora es una aproximación plana:
/// - Un grupo no se compone aparte: sus miembros se dibujan en su sitio y
///   heredan la opacidad del grupo (las máscaras llegarán con los grupos aparte).
/// - La coordenada Z y la cámara se ignoran.
pub fn evaluate(comp: &Composition, time: f64) -> Scene {
    let items = comp
        .layers()
        .iter()
        .filter(|l| l.is_active_at(time) && containers_active(comp, l, time))
        .filter_map(|l| item_for(comp, l, time))
        .collect();

    Scene { width: comp.width, height: comp.height, background: comp.background, items }
}

/// ¿Están activos (visibles y dentro de su ventana de tiempo) todos los grupos que contienen a la capa?
fn containers_active(comp: &Composition, layer: &Layer, time: f64) -> bool {
    let mut current = layer.container;
    // El límite protege frente a documentos corruptos con ciclos.
    for _ in 0..=comp.layers().len() {
        let Some(id) = current else { return true };
        let Some(group) = comp.layer(id) else { return true };
        if !group.is_active_at(time) {
            return false;
        }
        current = group.container;
    }
    false
}

fn item_for(comp: &Composition, layer: &Layer, time: f64) -> Option<Item> {
    let primitive = match &layer.kind {
        LayerKind::Shape(kind) => {
            let [width, height] = layer.size.value_at(time);
            Primitive::Shape { kind: kind.clone(), width, height }
        }
        // El resto de tipos se dibujará cuando tengan backend (texto, medios…).
        _ => return None,
    };

    let fill = match &layer.fill {
        Fill::None => None,
        Fill::Solid(color) => Some(color.value_at(time)),
    };

    Some(Item {
        layer: layer.id,
        primitive,
        transform: world_transform(comp, layer, time),
        opacity: effective_opacity(comp, layer, time),
        fill,
        blend: layer.blend,
    })
}

fn local_transform(layer: &Layer, time: f64) -> Affine {
    let t = &layer.transform;
    let [x, y, _z] = t.position.value_at(time);
    let [sx, sy] = t.scale.value_at(time);
    let [kx, ky] = t.skew.value_at(time);

    Affine::translate(x, y)
        .then_apply_to(Affine::rotate(t.rotation.value_at(time)))
        .then_apply_to(Affine::skew(kx, ky))
        .then_apply_to(Affine::scale(sx, sy))
}

/// Transformación local compuesta con la de toda la cadena de padres.
fn world_transform(comp: &Composition, layer: &Layer, time: f64) -> Affine {
    let mut world = local_transform(layer, time);
    let mut parent = layer.parent;
    for _ in 0..=comp.layers().len() {
        let Some(id) = parent else { break };
        let Some(p) = comp.layer(id) else { break };
        world = local_transform(p, time).then_apply_to(world);
        parent = p.parent;
    }
    world
}

fn effective_opacity(comp: &Composition, layer: &Layer, time: f64) -> f32 {
    let mut opacity = layer.opacity.value_at(time);
    let mut container = layer.container;
    for _ in 0..=comp.layers().len() {
        let Some(id) = container else { break };
        let Some(g) = comp.layer(id) else { break };
        opacity *= g.opacity.value_at(time);
        container = g.container;
    }
    opacity.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use shutter_core::{AspectRatio, Background, Resolution};

    fn comp() -> Composition {
        Composition::new(AspectRatio::R1x1, Resolution::P1080, 30, Background::Black)
    }

    fn rect() -> LayerKind {
        LayerKind::Shape(ShapeKind::Rectangle)
    }

    fn close(a: (f32, f32), b: (f32, f32)) -> bool {
        (a.0 - b.0).abs() < 1e-3 && (a.1 - b.1).abs() < 1e-3
    }

    #[test]
    fn affine_compose_applies_the_right_operand_first() {
        let t = Affine::translate(10.0, 0.0).then_apply_to(Affine::scale(2.0, 2.0));
        assert!(close(t.apply(1.0, 1.0), (12.0, 2.0)));
        let t = Affine::scale(2.0, 2.0).then_apply_to(Affine::translate(10.0, 0.0));
        assert!(close(t.apply(1.0, 1.0), (22.0, 2.0)));
    }

    #[test]
    fn rotation_is_clockwise_with_y_down() {
        // 90°: el eje X apunta hacia abajo en pantalla.
        assert!(close(Affine::rotate(90.0).apply(1.0, 0.0), (0.0, 1.0)));
    }

    #[test]
    fn skew_shears_along_each_axis() {
        assert!(close(Affine::skew(45.0, 0.0).apply(0.0, 1.0), (1.0, 1.0)));
        assert!(close(Affine::skew(0.0, 45.0).apply(1.0, 0.0), (1.0, 1.0)));
    }

    #[test]
    fn only_drawable_active_layers_become_items() {
        let mut c = comp();
        c.add_layer("shape", rect());
        c.add_layer("null", LayerKind::Null);
        let hidden = c.add_layer("hidden", rect());
        c.layer_mut(hidden).unwrap().visible = false;
        let late = c.add_layer("late", rect());
        c.layer_mut(late).unwrap().start = 3.0;

        let scene = evaluate(&c, 1.0);
        assert_eq!(scene.items.len(), 1);
        assert_eq!(evaluate(&c, 4.0).items.len(), 2);
    }

    #[test]
    fn items_keep_the_stacking_order() {
        let mut c = comp();
        let a = c.add_layer("a", rect());
        let b = c.add_layer("b", rect());
        let scene = evaluate(&c, 0.0);
        assert_eq!(scene.items.iter().map(|i| i.layer).collect::<Vec<_>>(), vec![a, b]);
    }

    #[test]
    fn child_transform_is_composed_with_its_parent() {
        let mut c = comp();
        let parent = c.add_layer("parent", LayerKind::Null);
        let child = c.add_layer("child", rect());
        c.layer_mut(parent).unwrap().transform.position.set_base([100.0, 100.0, 0.0]);
        c.layer_mut(child).unwrap().transform.position.set_base([50.0, 0.0, 0.0]);
        c.set_parent(child, Some(parent)).unwrap();

        let item = &evaluate(&c, 0.0).items[0];
        assert!(close(item.transform.apply(0.0, 0.0), (150.0, 100.0)));
    }

    #[test]
    fn rotating_the_parent_swings_the_child_around_it() {
        let mut c = comp();
        let parent = c.add_layer("parent", LayerKind::Null);
        let child = c.add_layer("child", rect());
        c.layer_mut(parent).unwrap().transform.position.set_base([100.0, 100.0, 0.0]);
        c.layer_mut(parent).unwrap().transform.rotation.set_base(90.0);
        c.layer_mut(child).unwrap().transform.position.set_base([50.0, 0.0, 0.0]);
        c.set_parent(child, Some(parent)).unwrap();

        let item = &evaluate(&c, 0.0).items[0];
        assert!(close(item.transform.apply(0.0, 0.0), (100.0, 150.0)));
    }

    #[test]
    fn animation_is_sampled_at_the_requested_time() {
        let mut c = comp();
        let id = c.add_layer("a", rect());
        let pos = &mut c.layer_mut(id).unwrap().transform.position;
        pos.set_key(0.0, [0.0, 0.0, 0.0]);
        pos.set_key(2.0, [200.0, 0.0, 0.0]);

        let item = &evaluate(&c, 1.0).items[0];
        assert!(close(item.transform.apply(0.0, 0.0), (100.0, 0.0)));
    }

    #[test]
    fn group_opacity_is_inherited_and_hidden_groups_hide_members() {
        let mut c = comp();
        let g = c.add_layer("g", LayerKind::Group);
        let a = c.add_layer("a", rect());
        c.set_container(a, Some(g)).unwrap();
        c.layer_mut(g).unwrap().opacity.set_base(0.5);
        c.layer_mut(a).unwrap().opacity.set_base(0.5);

        let scene = evaluate(&c, 0.0);
        assert_eq!(scene.items.len(), 1);
        assert!((scene.items[0].opacity - 0.25).abs() < 1e-6);

        c.layer_mut(g).unwrap().visible = false;
        assert!(evaluate(&c, 0.0).items.is_empty());
    }

    #[test]
    fn fill_none_has_no_color() {
        let mut c = comp();
        let id = c.add_layer("a", rect());
        c.layer_mut(id).unwrap().fill = Fill::None;
        assert_eq!(evaluate(&c, 0.0).items[0].fill, None);
    }
}
