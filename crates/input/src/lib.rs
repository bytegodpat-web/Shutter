//! Traduce eventos crudos de mouse y teclado a acciones semánticas del editor.
//!
//! La interfaz de referencia es táctil; aquí cada gesto tiene un equivalente
//! de escritorio:
//!
//! | Gesto táctil              | Escritorio                               |
//! |---------------------------|------------------------------------------|
//! | Tap                       | Click izquierdo → [`Action::Tap`]         |
//! | Tap prolongado            | Click derecho → [`Action::ContextMenu`]   |
//! | Doble tap                 | Doble click → [`Action::DoubleTap`]       |
//! | Arrastrar                 | Arrastre con click izquierdo              |
//! | Pellizco (zoom)           | Ctrl + scroll → [`Action::Zoom`]          |
//! | Desplazamiento horizontal | Flecha izquierda + scroll → [`Action::PanX`] |
//! | Desplazamiento vertical   | Flecha abajo + scroll → [`Action::PanY`]  |
//!
//! Como extra (configurable): Shift + scroll desplaza en horizontal y el botón
//! central arrastra el lienzo.
//!
//! El crate no depende de ningún sistema de ventanas: quien lo use convierte
//! sus eventos a [`RawEvent`] y consume las [`Action`] resultantes.

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    fn distance(self, other: Point) -> f32 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Teclas que el editor necesita distinguir. El resto se ignoran antes de llegar aquí.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    ArrowLeft,
    ArrowDown,
    Ctrl,
    Shift,
}

/// Evento de entrada ya independiente del sistema de ventanas.
/// `time_ms` es un reloj monótono en milisegundos.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RawEvent {
    KeyDown(Key),
    KeyUp(Key),
    ButtonDown {
        button: MouseButton,
        pos: Point,
        time_ms: u64,
    },
    ButtonUp {
        button: MouseButton,
        pos: Point,
        time_ms: u64,
    },
    Move {
        pos: Point,
    },
    /// `dy` positivo = rueda hacia arriba.
    Scroll {
        dx: f32,
        dy: f32,
        pos: Point,
    },
}

/// Acción semántica que consume la interfaz.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    Tap(Point),
    DoubleTap(Point),
    ContextMenu(Point),
    DragStart(Point),
    DragMove {
        pos: Point,
        delta: Point,
    },
    DragEnd(Point),
    /// Zoom alrededor de `pos`; `amount` positivo = acercar.
    Zoom {
        amount: f32,
        pos: Point,
    },
    PanX(f32),
    PanY(f32),
    /// Desplazamiento libre (arrastre con el botón central).
    Pan {
        dx: f32,
        dy: f32,
    },
    /// Scroll normal, sin modificadores.
    Scroll {
        dx: f32,
        dy: f32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Config {
    /// Shift + scroll desplaza en horizontal.
    pub shift_pans_horizontally: bool,
    /// Arrastrar con el botón central desplaza el lienzo.
    pub middle_button_pans: bool,
    /// Tiempo máximo entre dos clicks para contarlos como doble click.
    pub double_click_ms: u64,
    /// Píxeles que debe moverse el cursor para que un click pase a ser arrastre.
    pub drag_threshold: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            shift_pans_horizontally: true,
            middle_button_pans: true,
            double_click_ms: 350,
            drag_threshold: 4.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Press {
    button: MouseButton,
    start: Point,
    last: Point,
    dragging: bool,
}

#[derive(Debug, Default)]
pub struct Translator {
    config: Config,
    left: bool,
    down: bool,
    ctrl: bool,
    shift: bool,
    press: Option<Press>,
    last_tap: Option<(Point, u64)>,
}

impl Translator {
    pub fn new(config: Config) -> Self {
        Self { config, ..Self::default() }
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn set_config(&mut self, config: Config) {
        self.config = config;
    }

    /// Procesa un evento y devuelve las acciones que produce (normalmente 0 o 1).
    pub fn handle(&mut self, event: RawEvent) -> Vec<Action> {
        let mut out = Vec::new();
        match event {
            RawEvent::KeyDown(k) => self.set_key(k, true),
            RawEvent::KeyUp(k) => self.set_key(k, false),

            RawEvent::ButtonDown { button: MouseButton::Right, pos, .. } => {
                out.push(Action::ContextMenu(pos));
            }
            RawEvent::ButtonDown { button, pos, .. } => {
                self.press = Some(Press { button, start: pos, last: pos, dragging: false });
            }

            RawEvent::Move { pos } => self.on_move(pos, &mut out),

            RawEvent::ButtonUp { button, pos, time_ms } => {
                self.on_button_up(button, pos, time_ms, &mut out);
            }

            RawEvent::Scroll { dx, dy, pos } => self.on_scroll(dx, dy, pos, &mut out),
        }
        out
    }

    fn set_key(&mut self, key: Key, pressed: bool) {
        match key {
            Key::ArrowLeft => self.left = pressed,
            Key::ArrowDown => self.down = pressed,
            Key::Ctrl => self.ctrl = pressed,
            Key::Shift => self.shift = pressed,
        }
    }

    fn on_move(&mut self, pos: Point, out: &mut Vec<Action>) {
        let Some(press) = &mut self.press else { return };

        if !press.dragging && press.start.distance(pos) > self.config.drag_threshold {
            press.dragging = true;
            if press.button == MouseButton::Left {
                out.push(Action::DragStart(press.start));
            }
        }
        if !press.dragging {
            return;
        }

        let delta = Point::new(pos.x - press.last.x, pos.y - press.last.y);
        press.last = pos;
        match press.button {
            MouseButton::Left => out.push(Action::DragMove { pos, delta }),
            MouseButton::Middle if self.config.middle_button_pans => {
                out.push(Action::Pan { dx: delta.x, dy: delta.y });
            }
            _ => {}
        }
    }

    fn on_button_up(&mut self, button: MouseButton, pos: Point, time_ms: u64, out: &mut Vec<Action>) {
        let Some(press) = self.press else { return };
        if press.button != button {
            return;
        }
        self.press = None;

        if button != MouseButton::Left {
            return;
        }
        if press.dragging {
            out.push(Action::DragEnd(pos));
            return;
        }

        out.push(Action::Tap(pos));
        let is_double = self.last_tap.is_some_and(|(p, t)| {
            time_ms.saturating_sub(t) <= self.config.double_click_ms
                && p.distance(pos) <= self.config.drag_threshold
        });
        if is_double {
            out.push(Action::DoubleTap(pos));
            self.last_tap = None;
        } else {
            self.last_tap = Some((pos, time_ms));
        }
    }

    fn on_scroll(&mut self, dx: f32, dy: f32, pos: Point, out: &mut Vec<Action>) {
        // Un scroll horizontal puro (trackpad) también cuenta como cantidad.
        let amount = if dy != 0.0 { dy } else { dx };
        // Prioridad: Ctrl > flecha izquierda > flecha abajo > Shift.
        if self.ctrl {
            out.push(Action::Zoom { amount, pos });
        } else if self.left {
            out.push(Action::PanX(amount));
        } else if self.down {
            out.push(Action::PanY(amount));
        } else if self.shift && self.config.shift_pans_horizontally {
            out.push(Action::PanX(amount));
        } else {
            out.push(Action::Scroll { dx, dy });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const P: Point = Point::new(100.0, 100.0);

    fn down(button: MouseButton, pos: Point, t: u64) -> RawEvent {
        RawEvent::ButtonDown { button, pos, time_ms: t }
    }
    fn up(button: MouseButton, pos: Point, t: u64) -> RawEvent {
        RawEvent::ButtonUp { button, pos, time_ms: t }
    }
    fn scroll(dy: f32) -> RawEvent {
        RawEvent::Scroll { dx: 0.0, dy, pos: P }
    }

    #[test]
    fn left_click_is_tap() {
        let mut t = Translator::default();
        assert!(t.handle(down(MouseButton::Left, P, 0)).is_empty());
        assert_eq!(t.handle(up(MouseButton::Left, P, 50)), vec![Action::Tap(P)]);
    }

    #[test]
    fn right_click_opens_context_menu() {
        let mut t = Translator::default();
        assert_eq!(t.handle(down(MouseButton::Right, P, 0)), vec![Action::ContextMenu(P)]);
        // Soltar el botón derecho no produce nada más.
        assert!(t.handle(up(MouseButton::Right, P, 40)).is_empty());
    }

    #[test]
    fn two_quick_clicks_are_a_double_tap() {
        let mut t = Translator::default();
        t.handle(down(MouseButton::Left, P, 0));
        t.handle(up(MouseButton::Left, P, 40));
        t.handle(down(MouseButton::Left, P, 120));
        let out = t.handle(up(MouseButton::Left, P, 160));
        assert_eq!(out, vec![Action::Tap(P), Action::DoubleTap(P)]);
    }

    #[test]
    fn slow_second_click_is_not_a_double_tap() {
        let mut t = Translator::default();
        t.handle(down(MouseButton::Left, P, 0));
        t.handle(up(MouseButton::Left, P, 40));
        t.handle(down(MouseButton::Left, P, 1000));
        assert_eq!(t.handle(up(MouseButton::Left, P, 1040)), vec![Action::Tap(P)]);
    }

    #[test]
    fn third_click_does_not_chain_into_another_double_tap() {
        let mut t = Translator::default();
        for (d, u) in [(0, 40), (100, 140), (200, 240)] {
            t.handle(down(MouseButton::Left, P, d));
            let out = t.handle(up(MouseButton::Left, P, u));
            if d == 100 {
                assert!(out.contains(&Action::DoubleTap(P)));
            } else {
                assert!(!out.contains(&Action::DoubleTap(P)));
            }
        }
    }

    #[test]
    fn drag_emits_start_moves_and_end() {
        let mut t = Translator::default();
        t.handle(down(MouseButton::Left, P, 0));
        // Un movimiento pequeño todavía no es arrastre.
        assert!(t.handle(RawEvent::Move { pos: Point::new(101.0, 100.0) }).is_empty());

        let p2 = Point::new(120.0, 100.0);
        let out = t.handle(RawEvent::Move { pos: p2 });
        assert_eq!(out[0], Action::DragStart(P));
        assert_eq!(out[1], Action::DragMove { pos: p2, delta: Point::new(20.0, 0.0) });

        let p3 = Point::new(130.0, 110.0);
        assert_eq!(
            t.handle(RawEvent::Move { pos: p3 }),
            vec![Action::DragMove { pos: p3, delta: Point::new(10.0, 10.0) }]
        );
        assert_eq!(t.handle(up(MouseButton::Left, p3, 200)), vec![Action::DragEnd(p3)]);
    }

    #[test]
    fn dragging_does_not_produce_a_tap() {
        let mut t = Translator::default();
        t.handle(down(MouseButton::Left, P, 0));
        t.handle(RawEvent::Move { pos: Point::new(200.0, 200.0) });
        let out = t.handle(up(MouseButton::Left, Point::new(200.0, 200.0), 100));
        assert!(!out.iter().any(|a| matches!(a, Action::Tap(_))));
    }

    #[test]
    fn plain_scroll_passes_through() {
        let mut t = Translator::default();
        assert_eq!(t.handle(scroll(3.0)), vec![Action::Scroll { dx: 0.0, dy: 3.0 }]);
    }

    #[test]
    fn ctrl_scroll_zooms() {
        let mut t = Translator::default();
        t.handle(RawEvent::KeyDown(Key::Ctrl));
        assert_eq!(t.handle(scroll(2.0)), vec![Action::Zoom { amount: 2.0, pos: P }]);
        t.handle(RawEvent::KeyUp(Key::Ctrl));
        assert_eq!(t.handle(scroll(2.0)), vec![Action::Scroll { dx: 0.0, dy: 2.0 }]);
    }

    #[test]
    fn left_arrow_scroll_pans_horizontally() {
        let mut t = Translator::default();
        t.handle(RawEvent::KeyDown(Key::ArrowLeft));
        assert_eq!(t.handle(scroll(-5.0)), vec![Action::PanX(-5.0)]);
        t.handle(RawEvent::KeyUp(Key::ArrowLeft));
        assert_eq!(t.handle(scroll(-5.0)), vec![Action::Scroll { dx: 0.0, dy: -5.0 }]);
    }

    #[test]
    fn down_arrow_scroll_pans_vertically() {
        let mut t = Translator::default();
        t.handle(RawEvent::KeyDown(Key::ArrowDown));
        assert_eq!(t.handle(scroll(4.0)), vec![Action::PanY(4.0)]);
    }

    #[test]
    fn modifier_priority_is_ctrl_then_left_then_down() {
        let mut t = Translator::default();
        t.handle(RawEvent::KeyDown(Key::ArrowDown));
        t.handle(RawEvent::KeyDown(Key::ArrowLeft));
        assert_eq!(t.handle(scroll(1.0)), vec![Action::PanX(1.0)]);
        t.handle(RawEvent::KeyDown(Key::Ctrl));
        assert!(matches!(t.handle(scroll(1.0))[0], Action::Zoom { .. }));
    }

    #[test]
    fn shift_scroll_is_configurable() {
        let mut t = Translator::default();
        t.handle(RawEvent::KeyDown(Key::Shift));
        assert_eq!(t.handle(scroll(1.0)), vec![Action::PanX(1.0)]);

        t.set_config(Config { shift_pans_horizontally: false, ..Config::default() });
        assert_eq!(t.handle(scroll(1.0)), vec![Action::Scroll { dx: 0.0, dy: 1.0 }]);
    }

    #[test]
    fn horizontal_only_scroll_counts_as_amount() {
        let mut t = Translator::default();
        t.handle(RawEvent::KeyDown(Key::Ctrl));
        let out = t.handle(RawEvent::Scroll { dx: 6.0, dy: 0.0, pos: P });
        assert_eq!(out, vec![Action::Zoom { amount: 6.0, pos: P }]);
    }

    #[test]
    fn middle_button_drag_pans_the_canvas() {
        let mut t = Translator::default();
        t.handle(down(MouseButton::Middle, P, 0));
        let out = t.handle(RawEvent::Move { pos: Point::new(110.0, 100.0) });
        assert_eq!(out, vec![Action::Pan { dx: 10.0, dy: 0.0 }]);

        let mut off = Translator::new(Config { middle_button_pans: false, ..Config::default() });
        off.handle(down(MouseButton::Middle, P, 0));
        assert!(off.handle(RawEvent::Move { pos: Point::new(110.0, 100.0) }).is_empty());
    }

    #[test]
    fn move_without_press_does_nothing() {
        let mut t = Translator::default();
        assert!(t.handle(RawEvent::Move { pos: P }).is_empty());
    }
}
