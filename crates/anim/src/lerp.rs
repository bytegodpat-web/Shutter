/// Valores que se pueden interpolar linealmente.
///
/// `t` normalmente está en `[0, 1]`, pero las curvas con *overshoot*
/// (elástico, rebote) pueden producir valores fuera de ese rango.
pub trait Lerp: Clone {
    fn lerp(&self, other: &Self, t: f32) -> Self;
}

impl Lerp for f32 {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        self + (other - self) * t
    }
}

impl Lerp for f64 {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        self + (other - self) * f64::from(t)
    }
}

impl<const N: usize> Lerp for [f32; N] {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        let mut out = *self;
        for i in 0..N {
            out[i] = self[i].lerp(&other[i], t);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lerp_f32() {
        assert_eq!(0.0_f32.lerp(&10.0, 0.5), 5.0);
    }

    #[test]
    fn lerp_array() {
        let a = [0.0_f32, 100.0];
        let b = [10.0_f32, 0.0];
        assert_eq!(a.lerp(&b, 0.5), [5.0, 50.0]);
    }

    #[test]
    fn lerp_allows_overshoot() {
        assert_eq!(0.0_f32.lerp(&10.0, 1.2), 12.0);
    }
}
