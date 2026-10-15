use serde::{Deserialize, Serialize};
use std::f32::consts::PI;

/// Frecuencia (puntos de la retícula por unidad de tiempo) del ruido de [`Easing::Random`].
const NOISE_FREQ: f32 = 24.0;

/// Curva que remodela el progreso `t` (de 0 a 1) entre dos keyframes.
///
/// Todas las variantes cumplen `apply(0) == 0` y `apply(1) == 1`, pero algunas
/// (elástico, aleatorio) pueden salirse de `[0, 1]` por el camino (*overshoot*).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum Easing {
    /// Progreso uniforme.
    #[default]
    Linear,
    /// Curva de Bézier cúbica con puntos de control `(x1, y1)` y `(x2, y2)`.
    /// `x1` y `x2` se limitan a `[0, 1]` para que la curva sea una función del tiempo.
    CubicBezier { x1: f32, y1: f32, x2: f32, y2: f32 },
    /// Rebotes decrecientes que terminan en el valor final.
    Bounce { bounces: u32 },
    /// Oscilación amortiguada alrededor del valor final.
    Elastic { oscillations: f32, damping: f32 },
    /// Oscilación senoidal entre inicio y final. Usa `cycles` acabado en `.5`
    /// (por ejemplo `2.5`) para que la curva termine en el valor final.
    Cyclic { cycles: f32 },
    /// Tendencia lineal con ruido determinista (misma `seed`, misma curva).
    Random { seed: u32, amplitude: f32 },
    /// Escalones uniformes.
    Steps { count: u32 },
    /// Escalones con transiciones suaves; `smoothness` en `(0, 1]` es la
    /// fracción de cada escalón dedicada a la transición.
    ElasticSteps { count: u32, smoothness: f32 },
}

impl Easing {
    /// Entrada y salida suaves (equivale al `ease-in-out` habitual).
    pub const fn ease_in_out() -> Self {
        Easing::CubicBezier { x1: 0.42, y1: 0.0, x2: 0.58, y2: 1.0 }
    }

    /// Arranque suave.
    pub const fn ease_in() -> Self {
        Easing::CubicBezier { x1: 0.42, y1: 0.0, x2: 1.0, y2: 1.0 }
    }

    /// Final suave.
    pub const fn ease_out() -> Self {
        Easing::CubicBezier { x1: 0.0, y1: 0.0, x2: 0.58, y2: 1.0 }
    }

    /// Aplica la curva. `t` se limita a `[0, 1]`.
    pub fn apply(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match *self {
            Easing::Linear => t,
            Easing::CubicBezier { x1, y1, x2, y2 } => cubic_bezier(x1, y1, x2, y2, t),
            Easing::Bounce { bounces } => bounce(t, bounces),
            Easing::Elastic { oscillations, damping } => {
                // La envolvente (1 - t) fuerza que la curva termine exactamente en 1.
                1.0 - (-damping * t).exp() * (1.0 - t) * (2.0 * PI * oscillations * t).cos()
            }
            Easing::Cyclic { cycles } => (1.0 - (2.0 * PI * cycles * t).cos()) / 2.0,
            Easing::Random { seed, amplitude } => {
                let envelope = 4.0 * t * (1.0 - t);
                t + amplitude * value_noise(seed, t * NOISE_FREQ) * envelope
            }
            Easing::Steps { count } => {
                let n = count.max(1) as f32;
                (t * n).floor() / n
            }
            Easing::ElasticSteps { count, smoothness } => {
                let n = count.max(1) as f32;
                let smooth = smoothness.clamp(0.001, 1.0);
                let s = t * n;
                let k = s.floor();
                let f = s - k;
                let g = ((f - (1.0 - smooth)) / smooth).clamp(0.0, 1.0);
                let g = g * g * (3.0 - 2.0 * g);
                (k + g) / n
            }
        }
    }
}

fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32, x: f32) -> f32 {
    let x1 = x1.clamp(0.0, 1.0);
    let x2 = x2.clamp(0.0, 1.0);

    let cx = 3.0 * x1;
    let bx = 3.0 * (x2 - x1) - cx;
    let ax = 1.0 - cx - bx;
    let cy = 3.0 * y1;
    let by = 3.0 * (y2 - y1) - cy;
    let ay = 1.0 - cy - by;

    let sample_x = |s: f32| ((ax * s + bx) * s + cx) * s;
    let sample_dx = |s: f32| (3.0 * ax * s + 2.0 * bx) * s + cx;
    let sample_y = |s: f32| ((ay * s + by) * s + cy) * s;

    // Newton-Raphson; si no converge, bisección.
    let mut s = x;
    for _ in 0..8 {
        let err = sample_x(s) - x;
        if err.abs() < 1e-6 {
            return sample_y(s);
        }
        let d = sample_dx(s);
        if d.abs() < 1e-6 {
            break;
        }
        s -= err / d;
    }

    let (mut lo, mut hi) = (0.0_f32, 1.0_f32);
    s = x;
    for _ in 0..40 {
        let v = sample_x(s);
        if (v - x).abs() < 1e-6 {
            break;
        }
        if v < x {
            lo = s;
        } else {
            hi = s;
        }
        s = (lo + hi) / 2.0;
    }
    sample_y(s)
}

/// Caída seguida de `bounces` rebotes; cada rebote alcanza un cuarto de la altura anterior.
fn bounce(t: f32, bounces: u32) -> f32 {
    let n = bounces.max(1);

    // Con gravedad constante, caer desde la altura h tarda sqrt(h) y un rebote
    // de altura h (subir y bajar) tarda 2·sqrt(h).
    let mut total = 1.0_f32;
    let mut h = 1.0_f32;
    for _ in 0..n {
        h *= 0.25;
        total += 2.0 * h.sqrt();
    }

    let mut pos = t * total;
    if pos <= 1.0 {
        return pos * pos;
    }
    pos -= 1.0;

    let mut h = 1.0_f32;
    for i in 0..n {
        h *= 0.25;
        let dur = 2.0 * h.sqrt();
        if pos <= dur || i == n - 1 {
            let tau = ((pos / dur) * 2.0 - 1.0).clamp(-1.0, 1.0);
            return 1.0 - h * (1.0 - tau * tau);
        }
        pos -= dur;
    }
    1.0
}

fn hash(seed: u32, i: i32) -> f32 {
    let mut x = (i as u32).wrapping_mul(0x9E37_79B1) ^ seed.wrapping_mul(0x85EB_CA6B);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    x = x.wrapping_mul(0x297A_2D39);
    x ^= x >> 15;
    (x as f32 / u32::MAX as f32) * 2.0 - 1.0
}

/// Ruido de valor suavizado, en `[-1, 1]`.
fn value_noise(seed: u32, x: f32) -> f32 {
    let i = x.floor();
    let f = x - i;
    let a = hash(seed, i as i32);
    let b = hash(seed, i as i32 + 1);
    let s = f * f * (3.0 - 2.0 * f);
    a + (b - a) * s
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f32 = 1e-4;

    fn all_variants() -> Vec<Easing> {
        vec![
            Easing::Linear,
            Easing::ease_in_out(),
            Easing::ease_in(),
            Easing::ease_out(),
            Easing::Bounce { bounces: 4 },
            Easing::Elastic { oscillations: 3.0, damping: 4.0 },
            Easing::Cyclic { cycles: 2.5 },
            Easing::Random { seed: 7, amplitude: 0.2 },
            Easing::Steps { count: 5 },
            Easing::ElasticSteps { count: 5, smoothness: 0.4 },
        ]
    }

    #[test]
    fn every_curve_starts_at_zero_and_ends_at_one() {
        for e in all_variants() {
            assert!(e.apply(0.0).abs() < EPS, "{e:?} en t=0 da {}", e.apply(0.0));
            assert!((e.apply(1.0) - 1.0).abs() < EPS, "{e:?} en t=1 da {}", e.apply(1.0));
        }
    }

    #[test]
    fn input_is_clamped() {
        let e = Easing::Linear;
        assert_eq!(e.apply(-3.0), 0.0);
        assert_eq!(e.apply(3.0), 1.0);
    }

    #[test]
    fn bezier_with_diagonal_controls_is_identity() {
        let e = Easing::CubicBezier { x1: 0.25, y1: 0.25, x2: 0.75, y2: 0.75 };
        for i in 0..=10 {
            let t = i as f32 / 10.0;
            assert!((e.apply(t) - t).abs() < 1e-3);
        }
    }

    #[test]
    fn ease_in_out_is_symmetric_and_monotonic() {
        let e = Easing::ease_in_out();
        assert!((e.apply(0.5) - 0.5).abs() < 1e-3);
        let mut prev = -1.0;
        for i in 0..=100 {
            let v = e.apply(i as f32 / 100.0);
            assert!(v >= prev - 1e-6);
            prev = v;
        }
        assert!(e.apply(0.25) < 0.25);
        assert!(e.apply(0.75) > 0.75);
    }

    #[test]
    fn bounce_never_exceeds_one_and_bounces_back() {
        let e = Easing::Bounce { bounces: 4 };
        let mut touched_ground = 0;
        let mut prev = 0.0;
        for i in 0..=1000 {
            let v = e.apply(i as f32 / 1000.0);
            assert!(v <= 1.0 + EPS);
            if prev > 0.9 && v < prev && v < 0.95 {
                touched_ground += 1;
            }
            prev = v;
        }
        // Debe haber al menos una bajada tras llegar a 1 (un rebote).
        assert!(touched_ground >= 1);
    }

    #[test]
    fn elastic_overshoots() {
        let e = Easing::Elastic { oscillations: 3.0, damping: 4.0 };
        let max = (0..=1000).map(|i| e.apply(i as f32 / 1000.0)).fold(f32::MIN, f32::max);
        assert!(max > 1.0);
    }

    #[test]
    fn cyclic_oscillates() {
        let e = Easing::Cyclic { cycles: 2.5 };
        assert!((e.apply(0.2) - 1.0).abs() < 1e-3); // primer pico
        assert!(e.apply(0.4) < 0.01); // primer valle
    }

    #[test]
    fn steps_are_discrete() {
        let e = Easing::Steps { count: 4 };
        assert_eq!(e.apply(0.0), 0.0);
        assert_eq!(e.apply(0.24), 0.0);
        assert_eq!(e.apply(0.26), 0.25);
        assert_eq!(e.apply(0.76), 0.75);
        assert_eq!(e.apply(1.0), 1.0);
    }

    #[test]
    fn elastic_steps_ramp_between_steps() {
        let e = Easing::ElasticSteps { count: 4, smoothness: 0.5 };
        // Al principio de un escalón la curva está plana.
        assert!((e.apply(0.05) - 0.0).abs() < 1e-3);
        // A mitad de la transición está entre dos escalones.
        let mid = e.apply(0.2);
        assert!(mid > 0.0 && mid < 0.25);
    }

    #[test]
    fn random_is_deterministic_per_seed() {
        let a = Easing::Random { seed: 1, amplitude: 0.3 };
        let b = Easing::Random { seed: 1, amplitude: 0.3 };
        let c = Easing::Random { seed: 2, amplitude: 0.3 };
        assert_eq!(a.apply(0.37), b.apply(0.37));
        assert_ne!(a.apply(0.37), c.apply(0.37));
    }

    #[test]
    fn serde_roundtrip() {
        for e in all_variants() {
            let json = serde_json::to_string(&e).unwrap();
            let back: Easing = serde_json::from_str(&json).unwrap();
            assert_eq!(e, back);
        }
    }
}
