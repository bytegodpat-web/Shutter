//! Rasterizador por CPU de referencia, basado en `tiny-skia`.
//!
//! Sirve para vistas previas sin GPU, pruebas con píxeles y como referencia
//! contra la que comparar el backend de GPU.

use crate::scene::{Affine, Item, Primitive, Scene};
use shutter_core::{Background, BlendMode, ShapeKind};
use std::f32::consts::{FRAC_PI_2, TAU};
use tiny_skia::{Color, FillRule, Paint, Path, PathBuilder, Pixmap, Rect, Transform};

/// Imagen resultante, en RGBA de 8 bits con alfa **premultiplicado**.
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pixmap: Pixmap,
}

impl Frame {
    pub fn data(&self) -> &[u8] {
        self.pixmap.data()
    }

    /// RGBA premultiplicado del píxel `(x, y)`.
    ///
    /// # Panics
    /// Si `(x, y)` queda fuera de la imagen.
    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        assert!(x < self.width && y < self.height, "píxel fuera de la imagen");
        let i = ((y * self.width + x) * 4) as usize;
        let d = self.pixmap.data();
        [d[i], d[i + 1], d[i + 2], d[i + 3]]
    }

    /// Codifica la imagen como PNG.
    pub fn to_png(&self) -> Result<Vec<u8>, String> {
        self.pixmap.encode_png().map_err(|e| e.to_string())
    }
}

/// Dibuja una escena. Devuelve `None` si el tamaño del lienzo no es válido (0 píxeles).
pub fn render(scene: &Scene) -> Option<Frame> {
    let mut pixmap = Pixmap::new(scene.width, scene.height)?;
    pixmap.fill(background_color(scene.background));

    for item in &scene.items {
        draw_item(&mut pixmap, item);
    }
    Some(Frame { width: scene.width, height: scene.height, pixmap })
}

/// ¿Este rasterizador implementa el modo de mezcla? Los que no, se dibujan como `Normal`.
pub fn supports_blend(mode: BlendMode) -> bool {
    !matches!(
        mode,
        BlendMode::DarkerColor
            | BlendMode::LighterColor
            | BlendMode::LinearBurn
            | BlendMode::SoftOverlay
            | BlendMode::VividLight
            | BlendMode::LinearLight
            | BlendMode::PinLight
            | BlendMode::Subtract
            | BlendMode::Divide
            | BlendMode::Mask
            | BlendMode::Exclude
    )
}

fn background_color(bg: Background) -> Color {
    match bg {
        Background::Black => Color::BLACK,
        Background::LightGray => Color::from_rgba8(217, 217, 217, 255),
        Background::White => Color::WHITE,
        Background::Green => Color::from_rgba8(0, 255, 0, 255),
        Background::Blue => Color::from_rgba8(0, 0, 255, 255),
        Background::Transparent => Color::TRANSPARENT,
    }
}

fn to_skia_blend(mode: BlendMode) -> tiny_skia::BlendMode {
    use tiny_skia::BlendMode as S;
    match mode {
        BlendMode::Multiply => S::Multiply,
        BlendMode::Darken => S::Darken,
        BlendMode::ColorBurn => S::ColorBurn,
        BlendMode::Screen => S::Screen,
        BlendMode::Lighten => S::Lighten,
        BlendMode::ColorDodge => S::ColorDodge,
        BlendMode::LinearDodge => S::Plus,
        BlendMode::Overlay => S::Overlay,
        BlendMode::SoftLight => S::SoftLight,
        BlendMode::HardLight => S::HardLight,
        BlendMode::Difference => S::Difference,
        BlendMode::Exclusion => S::Exclusion,
        BlendMode::Hue => S::Hue,
        BlendMode::Saturation => S::Saturation,
        BlendMode::Color => S::Color,
        BlendMode::Luminosity => S::Luminosity,
        _ => S::SourceOver,
    }
}

fn to_skia_transform(t: Affine) -> Transform {
    Transform::from_row(t.a, t.b, t.c, t.d, t.e, t.f)
}

fn draw_item(pixmap: &mut Pixmap, item: &Item) {
    let Some([r, g, b, a]) = item.fill else { return };
    let alpha = (a * item.opacity).clamp(0.0, 1.0);
    if alpha <= 0.0 {
        return;
    }
    let Some(color) = Color::from_rgba(r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0), alpha) else {
        return;
    };

    let Primitive::Shape { kind, width, height } = &item.primitive;
    let Some(path) = shape_path(kind, *width, *height) else { return };

    let mut paint = Paint::default();
    paint.set_color(color);
    paint.anti_alias = true;
    paint.blend_mode = to_skia_blend(item.blend);

    pixmap.fill_path(&path, &paint, FillRule::Winding, to_skia_transform(item.transform), None);
}

/// Contorno de la forma, centrado en el origen.
fn shape_path(kind: &ShapeKind, width: f32, height: f32) -> Option<Path> {
    let (w, h) = (width.abs(), height.abs());
    if w == 0.0 || h == 0.0 {
        return None;
    }
    match kind {
        ShapeKind::Rectangle => Some(PathBuilder::from_rect(Rect::from_xywh(-w / 2.0, -h / 2.0, w, h)?)),
        ShapeKind::Ellipse => PathBuilder::from_oval(Rect::from_xywh(-w / 2.0, -h / 2.0, w, h)?),
        ShapeKind::Polygon { sides } => {
            let n = (*sides).max(3);
            let points = (0..n).map(|k| vertex(k as f32 / n as f32, w / 2.0, h / 2.0));
            polygon(points)
        }
        ShapeKind::Star { points, inner_ratio } => {
            let n = (*points).max(2);
            let inner = inner_ratio.clamp(0.05, 1.0);
            let vertices = (0..n * 2).map(|k| {
                let radius = if k % 2 == 0 { 1.0 } else { inner };
                vertex(k as f32 / (n * 2) as f32, radius * w / 2.0, radius * h / 2.0)
            });
            polygon(vertices)
        }
    }
}

/// Vértice a la fracción `turn` de vuelta, empezando arriba y avanzando en sentido horario.
fn vertex(turn: f32, rx: f32, ry: f32) -> (f32, f32) {
    let angle = turn * TAU - FRAC_PI_2;
    (rx * angle.cos(), ry * angle.sin())
}

fn polygon(mut points: impl Iterator<Item = (f32, f32)>) -> Option<Path> {
    let mut pb = PathBuilder::new();
    let (x, y) = points.next()?;
    pb.move_to(x, y);
    for (x, y) in points {
        pb.line_to(x, y);
    }
    pb.close();
    pb.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{evaluate, Scene};
    use shutter_core::{AspectRatio, Composition, Fill, LayerKind, Resolution};

    const BLACK: [u8; 4] = [0, 0, 0, 255];
    const RED: [u8; 4] = [255, 0, 0, 255];

    /// Lienzo de 180×180 con el centro en (90, 90).
    fn comp() -> Composition {
        Composition::new(AspectRatio::R1x1, Resolution::P180, 30, Background::Black)
    }

    fn shape(c: &mut Composition, kind: ShapeKind, w: f32, h: f32) -> u64 {
        let id = c.add_layer("s", LayerKind::Shape(kind));
        let l = c.layer_mut(id).unwrap();
        l.size.set_base([w, h]);
        l.fill = Fill::Solid(shutter_anim::Property::new([1.0, 0.0, 0.0, 1.0]));
        id
    }

    fn draw(c: &Composition, t: f64) -> Frame {
        render(&evaluate(c, t)).unwrap()
    }

    #[test]
    fn rectangle_fills_its_area_only() {
        let mut c = comp();
        shape(&mut c, ShapeKind::Rectangle, 60.0, 60.0);
        let f = draw(&c, 0.0);
        assert_eq!(f.pixel(90, 90), RED);
        assert_eq!(f.pixel(65, 65), RED);
        assert_eq!(f.pixel(55, 90), BLACK);
        assert_eq!(f.pixel(5, 5), BLACK);
    }

    #[test]
    fn ellipse_leaves_the_bounding_box_corners_empty() {
        let mut c = comp();
        shape(&mut c, ShapeKind::Ellipse, 100.0, 100.0);
        let f = draw(&c, 0.0);
        assert_eq!(f.pixel(90, 90), RED);
        assert_eq!(f.pixel(44, 44), BLACK); // esquina de la caja, fuera del círculo
        assert_eq!(f.pixel(90, 45), RED); // borde superior, dentro
    }

    #[test]
    fn polygon_points_up() {
        let mut c = comp();
        shape(&mut c, ShapeKind::Polygon { sides: 3 }, 100.0, 100.0);
        let f = draw(&c, 0.0);
        // Un triángulo con el vértice arriba: ancho arriba casi nulo, ancho abajo grande.
        assert_eq!(f.pixel(60, 90 - 45), BLACK);
        assert_eq!(f.pixel(60, 110), RED);
    }

    #[test]
    fn star_has_a_narrow_waist() {
        let mut c = comp();
        shape(&mut c, ShapeKind::Star { points: 5, inner_ratio: 0.4 }, 120.0, 120.0);
        let f = draw(&c, 0.0);
        assert_eq!(f.pixel(90, 90), RED);
        assert_eq!(f.pixel(90, 40), RED); // dentro de la punta superior
                                          // Entre la punta superior y la siguiente está el vértice interior (radio 24);
                                          // a radio 40 en esa dirección ya estamos fuera de la estrella.
        assert_eq!(f.pixel(114, 58), BLACK);
    }

    #[test]
    fn opacity_blends_with_the_background() {
        let mut c = comp();
        let id = shape(&mut c, ShapeKind::Rectangle, 60.0, 60.0);
        c.layer_mut(id).unwrap().fill = Fill::Solid(shutter_anim::Property::new([1.0, 1.0, 1.0, 1.0]));
        c.layer_mut(id).unwrap().opacity.set_base(0.5);
        let p = draw(&c, 0.0).pixel(90, 90);
        assert!((i32::from(p[0]) - 128).abs() <= 2, "{p:?}");
        assert_eq!(p[3], 255);
    }

    #[test]
    fn transparent_background_has_zero_alpha() {
        let mut c = Composition::new(AspectRatio::R1x1, Resolution::P180, 30, Background::Transparent);
        shape(&mut c, ShapeKind::Rectangle, 40.0, 40.0);
        let f = draw(&c, 0.0);
        assert_eq!(f.pixel(5, 5)[3], 0);
        assert_eq!(f.pixel(90, 90), RED);
    }

    #[test]
    fn background_colors() {
        let mut c = comp();
        c.background = Background::White;
        assert_eq!(draw(&c, 0.0).pixel(0, 0), [255, 255, 255, 255]);
        c.background = Background::Blue;
        assert_eq!(draw(&c, 0.0).pixel(0, 0), [0, 0, 255, 255]);
    }

    #[test]
    fn animated_position_moves_the_shape() {
        let mut c = comp();
        let id = shape(&mut c, ShapeKind::Rectangle, 20.0, 20.0);
        let pos = &mut c.layer_mut(id).unwrap().transform.position;
        pos.set_key(0.0, [30.0, 90.0, 0.0]);
        pos.set_key(1.0, [150.0, 90.0, 0.0]);

        let start = draw(&c, 0.0);
        let end = draw(&c, 1.0);
        assert_eq!(start.pixel(30, 90), RED);
        assert_eq!(start.pixel(150, 90), BLACK);
        assert_eq!(end.pixel(30, 90), BLACK);
        assert_eq!(end.pixel(150, 90), RED);
    }

    #[test]
    fn rotating_the_parent_rotates_the_child() {
        let mut c = comp();
        let id = shape(&mut c, ShapeKind::Rectangle, 100.0, 20.0);
        let pivot = c.add_layer("pivot", LayerKind::Null);
        c.set_parent(id, Some(pivot)).unwrap();
        // El hijo queda centrado en el pivote; el pivote gira 90°.
        c.layer_mut(id).unwrap().transform.position.set_base([0.0, 0.0, 0.0]);
        c.layer_mut(pivot).unwrap().transform.position.set_base([90.0, 90.0, 0.0]);
        c.layer_mut(pivot).unwrap().transform.rotation.set_base(90.0);

        let f = draw(&c, 0.0);
        assert_eq!(f.pixel(90, 90 + 40), RED); // ahora es vertical
        assert_eq!(f.pixel(90 + 40, 90), BLACK);
    }

    #[test]
    fn layers_are_painted_bottom_to_top() {
        let mut c = comp();
        shape(&mut c, ShapeKind::Rectangle, 80.0, 80.0);
        let top = shape(&mut c, ShapeKind::Rectangle, 40.0, 40.0);
        c.layer_mut(top).unwrap().fill = Fill::Solid(shutter_anim::Property::new([0.0, 1.0, 0.0, 1.0]));
        let f = draw(&c, 0.0);
        assert_eq!(f.pixel(90, 90), [0, 255, 0, 255]);
        assert_eq!(f.pixel(60, 90), RED);
    }

    #[test]
    fn multiply_darkens_and_unsupported_modes_fall_back_to_normal() {
        assert!(supports_blend(BlendMode::Multiply));
        assert!(!supports_blend(BlendMode::Mask));

        let mut c = comp();
        c.background = Background::White;
        let id = shape(&mut c, ShapeKind::Rectangle, 60.0, 60.0);
        c.layer_mut(id).unwrap().blend = BlendMode::Multiply;
        // rojo × blanco = rojo
        assert_eq!(draw(&c, 0.0).pixel(90, 90), RED);
        // rojo × verde = negro
        c.background = Background::Green;
        assert_eq!(draw(&c, 0.0).pixel(90, 90), BLACK);
    }

    #[test]
    fn zero_sized_canvas_is_rejected() {
        let scene = Scene { width: 0, height: 10, background: Background::Black, items: vec![] };
        assert!(render(&scene).is_none());
    }

    #[test]
    fn png_has_the_right_signature() {
        let png = draw(&comp(), 0.0).to_png().unwrap();
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    }
}
