//! Genera unos fotogramas PNG de una composición animada de ejemplo.
//!
//! ```bash
//! cargo run -p shutter-render --example demo -- salida/
//! ```

use shutter_anim::{Easing, Property};
use shutter_core::{AspectRatio, Background, Composition, Fill, LayerKind, Resolution, ShapeKind};
use std::{env, fs, path::PathBuf};

fn solid(r: u8, g: u8, b: u8) -> Fill {
    Fill::Solid(Property::new([r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0]))
}

fn main() {
    let out = PathBuf::from(env::args().nth(1).unwrap_or_else(|| "demo-out".into()));
    fs::create_dir_all(&out).expect("no se pudo crear la carpeta de salida");

    let mut c = Composition::new(AspectRatio::R16x9, Resolution::P540, 30, Background::LightGray);
    c.duration = 4.0;
    let (cx, cy) = (c.width as f32 / 2.0, c.height as f32 / 2.0);

    // Estrella central: crece con rebote y gira con suavidad.
    let star = c.add_layer("estrella", LayerKind::Shape(ShapeKind::Star { points: 5, inner_ratio: 0.45 }));
    let l = c.layer_mut(star).unwrap();
    l.size.set_base([180.0, 180.0]);
    l.fill = solid(0xF2, 0xB7, 0x05);
    l.transform.scale.set_key_with_easing(0.0, [0.2, 0.2], Some(Easing::Bounce { bounces: 3 }));
    l.transform.scale.set_key(1.5, [1.0, 1.0]);
    l.transform.rotation.set_key_with_easing(0.0, 0.0, Some(Easing::ease_in_out()));
    l.transform.rotation.set_key(4.0, 144.0);

    // Un nulo que gira y del que cuelgan dos satélites (jerarquía padre-hijo).
    let pivot = c.add_layer("pivote", LayerKind::Null);
    let l = c.layer_mut(pivot).unwrap();
    l.transform.position.set_base([cx, cy, 0.0]);
    l.transform.rotation.set_key(0.0, 0.0);
    l.transform.rotation.set_key(4.0, 360.0);

    let orbit = c.add_layer("circulo", LayerKind::Shape(ShapeKind::Ellipse));
    let l = c.layer_mut(orbit).unwrap();
    l.size.set_base([70.0, 70.0]);
    l.fill = solid(0x47, 0x92, 0xE1);
    l.transform.position.set_base([190.0, 0.0, 0.0]);
    c.set_parent(orbit, Some(pivot)).unwrap();

    let hexagon = c.add_layer("hexagono", LayerKind::Shape(ShapeKind::Polygon { sides: 6 }));
    let l = c.layer_mut(hexagon).unwrap();
    l.size.set_base([90.0, 90.0]);
    l.fill = solid(0xD9, 0x3F, 0x6B);
    l.transform.position.set_base([-190.0, 0.0, 0.0]);
    l.transform.rotation.set_key(0.0, 0.0);
    l.transform.rotation.set_key(4.0, -720.0);
    c.set_parent(hexagon, Some(pivot)).unwrap();

    // Un rectángulo que cruza el lienzo y se desvanece.
    let bar = c.add_layer("barra", LayerKind::Shape(ShapeKind::Rectangle));
    let l = c.layer_mut(bar).unwrap();
    l.size.set_base([240.0, 26.0]);
    l.fill = solid(0x2B, 0x2D, 0x42);
    l.transform.position.set_key_with_easing(0.0, [-120.0, 60.0, 0.0], Some(Easing::ease_out()));
    l.transform.position.set_key(2.0, [cx, 60.0, 0.0]);
    l.opacity.set_key(2.0, 1.0);
    l.opacity.set_key(4.0, 0.0);

    for (i, t) in [0.0, 0.5, 1.5, 3.0].into_iter().enumerate() {
        let frame = shutter_render::render_at(&c, t).expect("lienzo válido");
        let path = out.join(format!("frame_{i}_{:.1}s.png", t));
        fs::write(&path, frame.to_png().expect("PNG")).expect("no se pudo escribir el PNG");
        println!("{}", path.display());
    }
}
