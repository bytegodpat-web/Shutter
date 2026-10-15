# Shutter

A Alight Motion alternative for pc

Editor de motion graphics y video para **escritorio** (Windows, macOS y Linux), escrito en Rust.

> **Estado:** muy temprano. Por ahora solo existe el modelo del documento, el motor de animación y la
> capa de entrada. Todavía no hay interfaz, render ni exportación.

## Objetivo

Una herramienta de motion graphics con capas, keyframes, curvas de easing, efectos y exportación de
video, con una interfaz inspirada en las apps móviles de edición, pero pensada para mouse y teclado.

## Estructura

| Crate | Qué hace |
|---|---|
| [`shutter-anim`](crates/anim) | Keyframes, interpolación y curvas de easing (Bézier, rebote, elástico, cíclico, aleatorio, pasos). |
| [`shutter-core`](crates/core) | Modelo del documento: proyecto, composición, capas, jerarquía, grupos y modos de mezcla. |
| [`shutter-input`](crates/input) | Traduce eventos de mouse y teclado a acciones del editor (ver abajo). |

Próximos crates previstos: `render` (wgpu), `media`, `effects`, `export` y `ui`.

## Entrada: gestos táctiles en escritorio

| Gesto táctil | Escritorio |
|---|---|
| Tap | Click izquierdo |
| Tap prolongado | Click derecho (menú contextual) |
| Doble tap | Doble click |
| Pellizco (zoom) | `Ctrl` + scroll |
| Desplazamiento horizontal | Flecha izquierda mantenida + scroll |
| Desplazamiento vertical | Flecha abajo mantenida + scroll |

Alternativas configurables: `Shift` + scroll (horizontal) y arrastrar con el botón central (mover el lienzo).

## Desarrollo

```bash
cargo test --workspace
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
```

## Documentación

Ver [`docs/`](docs/) y [`CONTRIBUTING.md`](CONTRIBUTING.md).

## Licencia

[Apache 2.0](LICENSE).
