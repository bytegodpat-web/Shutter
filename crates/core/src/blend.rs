use serde::{Deserialize, Serialize};

/// Familias en las que se agrupan los modos de mezcla en la interfaz.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlendGroup {
    Normal,
    Darken,
    Lighten,
    Contrast,
    Difference,
    Color,
    Mask,
}

/// Cómo se combina una capa con lo que hay debajo.
///
/// `Mask` y `Exclude` no mezclan colores: usan la forma de la capa como
/// máscara dentro de su grupo (ver el documento de especificación).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlendMode {
    #[default]
    Normal,
    // Oscurecer
    Multiply,
    Darken,
    DarkerColor,
    ColorBurn,
    LinearBurn,
    // Aclarar
    Screen,
    Lighten,
    LighterColor,
    ColorDodge,
    LinearDodge,
    // Contraste
    Overlay,
    SoftLight,
    HardLight,
    SoftOverlay,
    VividLight,
    LinearLight,
    PinLight,
    // Diferencia
    Difference,
    Exclusion,
    Subtract,
    Divide,
    // Color
    Hue,
    Saturation,
    Color,
    Luminosity,
    // Enmascarar
    Mask,
    Exclude,
}

impl BlendMode {
    pub const ALL: [BlendMode; 28] = [
        BlendMode::Normal,
        BlendMode::Multiply,
        BlendMode::Darken,
        BlendMode::DarkerColor,
        BlendMode::ColorBurn,
        BlendMode::LinearBurn,
        BlendMode::Screen,
        BlendMode::Lighten,
        BlendMode::LighterColor,
        BlendMode::ColorDodge,
        BlendMode::LinearDodge,
        BlendMode::Overlay,
        BlendMode::SoftLight,
        BlendMode::HardLight,
        BlendMode::SoftOverlay,
        BlendMode::VividLight,
        BlendMode::LinearLight,
        BlendMode::PinLight,
        BlendMode::Difference,
        BlendMode::Exclusion,
        BlendMode::Subtract,
        BlendMode::Divide,
        BlendMode::Hue,
        BlendMode::Saturation,
        BlendMode::Color,
        BlendMode::Luminosity,
        BlendMode::Mask,
        BlendMode::Exclude,
    ];

    pub fn group(self) -> BlendGroup {
        use BlendMode::*;
        match self {
            Normal => BlendGroup::Normal,
            Multiply | Darken | DarkerColor | ColorBurn | LinearBurn => BlendGroup::Darken,
            Screen | Lighten | LighterColor | ColorDodge | LinearDodge => BlendGroup::Lighten,
            Overlay | SoftLight | HardLight | SoftOverlay | VividLight | LinearLight | PinLight => {
                BlendGroup::Contrast
            }
            Difference | Exclusion | Subtract | Divide => BlendGroup::Difference,
            Hue | Saturation | Color | Luminosity => BlendGroup::Color,
            Mask | Exclude => BlendGroup::Mask,
        }
    }

    /// `true` si la capa actúa como máscara en lugar de mezclar colores.
    pub fn is_mask(self) -> bool {
        self.group() == BlendGroup::Mask
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count(g: BlendGroup) -> usize {
        BlendMode::ALL.iter().filter(|m| m.group() == g).count()
    }

    #[test]
    fn groups_have_the_expected_sizes() {
        assert_eq!(count(BlendGroup::Normal), 1);
        assert_eq!(count(BlendGroup::Darken), 5);
        assert_eq!(count(BlendGroup::Lighten), 5);
        assert_eq!(count(BlendGroup::Contrast), 7);
        assert_eq!(count(BlendGroup::Difference), 4);
        assert_eq!(count(BlendGroup::Color), 4);
        assert_eq!(count(BlendGroup::Mask), 2);
    }

    #[test]
    fn all_lists_every_mode_once() {
        let mut v = BlendMode::ALL.to_vec();
        v.dedup();
        assert_eq!(v.len(), 28);
    }

    #[test]
    fn only_mask_modes_are_masks() {
        assert!(BlendMode::Mask.is_mask());
        assert!(BlendMode::Exclude.is_mask());
        assert!(!BlendMode::Multiply.is_mask());
    }
}
