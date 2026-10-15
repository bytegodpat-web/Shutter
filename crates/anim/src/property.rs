use crate::{Easing, Lerp};
use serde::{Deserialize, Serialize};

/// Dos tiempos más cerca que esto (en segundos) se consideran el mismo instante.
const TIME_EPS: f64 = 1e-6;

/// Valor fijado en un instante. `easing` remodela el tramo que va desde este
/// keyframe hasta el siguiente.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Keyframe<T> {
    /// Tiempo en segundos.
    pub time: f64,
    pub value: T,
    pub easing: Easing,
}

/// Valor que puede animarse con keyframes.
///
/// Sin keyframes devuelve siempre el valor base. Con keyframes, antes del
/// primero y después del último se mantiene el valor de ese extremo.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Property<T> {
    base: T,
    keys: Vec<Keyframe<T>>,
}

impl<T: Lerp> Property<T> {
    pub fn new(base: T) -> Self {
        Self { base, keys: Vec::new() }
    }

    /// Valor sin animar (el que se usa cuando no hay keyframes).
    pub fn base(&self) -> &T {
        &self.base
    }

    pub fn is_animated(&self) -> bool {
        !self.keys.is_empty()
    }

    /// Keyframes ordenados por tiempo.
    pub fn keys(&self) -> &[Keyframe<T>] {
        &self.keys
    }

    /// Keyframe situado en `time`, si existe.
    pub fn key_at(&self, time: f64) -> Option<&Keyframe<T>> {
        self.keys.iter().find(|k| (k.time - time).abs() < TIME_EPS)
    }

    /// Valor en el instante `time` (segundos).
    pub fn value_at(&self, time: f64) -> T {
        let (Some(first), Some(last)) = (self.keys.first(), self.keys.last()) else {
            return self.base.clone();
        };
        if time <= first.time {
            return first.value.clone();
        }
        if time >= last.time {
            return last.value.clone();
        }

        // `partition_point` devuelve el primer keyframe con tiempo > `time`.
        let next = self.keys.partition_point(|k| k.time <= time);
        let (a, b) = (&self.keys[next - 1], &self.keys[next]);
        let u = ((time - a.time) / (b.time - a.time)) as f32;
        a.value.lerp(&b.value, a.easing.apply(u))
    }

    /// Crea o actualiza el keyframe en `time`. Si ya existía conserva su easing.
    pub fn set_key(&mut self, time: f64, value: T) {
        self.set_key_with_easing(time, value, None);
    }

    /// Como [`set_key`](Self::set_key), pero fijando también el easing.
    pub fn set_key_with_easing(&mut self, time: f64, value: T, easing: Option<Easing>) {
        if let Some(k) = self.keys.iter_mut().find(|k| (k.time - time).abs() < TIME_EPS) {
            k.value = value;
            if let Some(e) = easing {
                k.easing = e;
            }
            return;
        }
        let key = Keyframe { time, value, easing: easing.unwrap_or_default() };
        let at = self.keys.partition_point(|k| k.time < time);
        self.keys.insert(at, key);
    }

    /// Cambia el easing del keyframe en `time`. Devuelve `false` si no existe.
    pub fn set_easing(&mut self, time: f64, easing: Easing) -> bool {
        match self.keys.iter_mut().find(|k| (k.time - time).abs() < TIME_EPS) {
            Some(k) => {
                k.easing = easing;
                true
            }
            None => false,
        }
    }

    /// Elimina el keyframe en `time`. Devuelve `false` si no existe.
    pub fn remove_key(&mut self, time: f64) -> bool {
        let before = self.keys.len();
        self.keys.retain(|k| (k.time - time).abs() >= TIME_EPS);
        self.keys.len() != before
    }

    /// Edición desde la interfaz (por ejemplo, arrastrar una capa con el mouse).
    ///
    /// Si la propiedad está animada crea o actualiza el keyframe en `time`;
    /// si no, cambia el valor base.
    pub fn edit_at(&mut self, time: f64, value: T) {
        if self.is_animated() {
            self.set_key(time, value);
        } else {
            self.base = value;
        }
    }

    /// Cambia el valor base sin tocar los keyframes.
    pub fn set_base(&mut self, value: T) {
        self.base = value;
    }
}

impl<T: Lerp + Default> Default for Property<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn without_keys_returns_base() {
        let p = Property::new(3.0_f32);
        assert_eq!(p.value_at(0.0), 3.0);
        assert_eq!(p.value_at(99.0), 3.0);
        assert!(!p.is_animated());
    }

    #[test]
    fn holds_value_outside_the_keys() {
        let mut p = Property::new(0.0_f32);
        p.set_key(1.0, 10.0);
        p.set_key(2.0, 20.0);
        assert_eq!(p.value_at(0.0), 10.0);
        assert_eq!(p.value_at(5.0), 20.0);
    }

    #[test]
    fn interpolates_linearly_by_default() {
        let mut p = Property::new(0.0_f32);
        p.set_key(0.0, 0.0);
        p.set_key(2.0, 100.0);
        assert_eq!(p.value_at(1.0), 50.0);
        assert_eq!(p.value_at(0.5), 25.0);
    }

    #[test]
    fn keys_are_kept_sorted() {
        let mut p = Property::new(0.0_f32);
        p.set_key(3.0, 3.0);
        p.set_key(1.0, 1.0);
        p.set_key(2.0, 2.0);
        let times: Vec<f64> = p.keys().iter().map(|k| k.time).collect();
        assert_eq!(times, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn set_key_on_existing_time_updates_and_keeps_easing() {
        let mut p = Property::new(0.0_f32);
        p.set_key_with_easing(1.0, 5.0, Some(Easing::Steps { count: 2 }));
        p.set_key(1.0, 9.0);
        assert_eq!(p.keys().len(), 1);
        assert_eq!(p.key_at(1.0).unwrap().value, 9.0);
        assert_eq!(p.key_at(1.0).unwrap().easing, Easing::Steps { count: 2 });
    }

    #[test]
    fn easing_of_a_key_shapes_the_next_segment() {
        let mut p = Property::new(0.0_f32);
        p.set_key_with_easing(0.0, 0.0, Some(Easing::Steps { count: 2 }));
        p.set_key(1.0, 100.0);
        assert_eq!(p.value_at(0.25), 0.0);
        assert_eq!(p.value_at(0.75), 50.0);
    }

    #[test]
    fn set_easing_and_remove_key() {
        let mut p = Property::new(0.0_f32);
        p.set_key(1.0, 1.0);
        assert!(p.set_easing(1.0, Easing::ease_in_out()));
        assert!(!p.set_easing(2.0, Easing::Linear));
        assert!(p.remove_key(1.0));
        assert!(!p.remove_key(1.0));
        assert!(!p.is_animated());
    }

    #[test]
    fn edit_at_changes_base_when_not_animated() {
        let mut p = Property::new(1.0_f32);
        p.edit_at(5.0, 7.0);
        assert_eq!(*p.base(), 7.0);
        assert!(!p.is_animated());
    }

    #[test]
    fn edit_at_creates_keyframe_when_animated() {
        let mut p = Property::new(0.0_f32);
        p.set_key(0.0, 0.0);
        p.edit_at(2.0, 40.0);
        assert_eq!(p.keys().len(), 2);
        assert_eq!(p.value_at(2.0), 40.0);
        assert_eq!(*p.base(), 0.0);
    }

    #[test]
    fn works_with_vectors() {
        let mut p = Property::new([0.0_f32, 0.0]);
        p.set_key(0.0, [0.0, 0.0]);
        p.set_key(1.0, [10.0, 20.0]);
        assert_eq!(p.value_at(0.5), [5.0, 10.0]);
    }

    #[test]
    fn serde_roundtrip() {
        let mut p = Property::new(1.0_f32);
        p.set_key_with_easing(0.0, 0.0, Some(Easing::Bounce { bounces: 3 }));
        p.set_key(1.0, 2.0);
        let json = serde_json::to_string(&p).unwrap();
        let back: Property<f32> = serde_json::from_str(&json).unwrap();
        assert_eq!(p, back);
    }
}
