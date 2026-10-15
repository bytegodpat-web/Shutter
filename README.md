# Shutter

A Alight Motion alternative for pc

Editor de motion graphics y video para **escritorio** (Windows, macOS y Linux), escrito en Rust.

> **Estado:** muy temprano. Existen el modelo del documento, el motor de animación, la capa de entrada y
> un rasterizador por CPU que dibuja formas animadas. Todavía no hay interfaz, render en GPU ni exportación.

## Objetivo

Una herramienta de motion graphics con capas, keyframes, curvas de easing, efectos y exportación de
video, con una interfaz inspirada en las apps móviles de edición, pero pensada para mouse y teclado.

## Estructura

| Crate | Qué hace |
|---|---|
| [`shutter-anim`](crates/anim) | Keyframes, interpolación y curvas de easing (Bézier, rebote, elástico, cíclico, aleatorio, pasos). |
| [`shutter-core`](crates/core) | Modelo del documento: proyecto, composición, capas, jerarquía, grupos y modos de mezcla. |
| [`shutter-render`](crates/render) | Evalúa una composición en un instante (jerarquía, animación, opacidad) y la dibuja con un rasterizador por CPU de referencia. |
| [`shutter-input`](crates/input) | Traduce eventos de mouse y teclado a acciones del editor (ver abajo). |

Próximos crates previstos: backend `wgpu` para `render`, `media`, `effects`, `export` y `ui`.

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

## Probar el render

```bash
cargo run -p shutter-render --example demo -- salida/
```

Genera unos fotogramas PNG de una composición animada de ejemplo.

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
