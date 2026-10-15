# Documentación

## Arquitectura y desarrollo

- [Arquitectura del backend wgpu](ARQUITECTURA_WGPU.md): pipeline de renderizado, geometría vectorial, grupos, máscaras, modos de mezcla, readback, integración con egui y comparación CPU/GPU.
- [Plan de hitos](PLAN_HITOS.md): tareas pequeñas con criterios de terminado desde el backend GPU hasta la exportación MP4.
- [README del proyecto](../README.md): estado general, crates y comandos de desarrollo.

## Estado de los hitos

0. `shutter-input`: traducción de gestos (hecho).
1. `shutter-core` y `shutter-anim`: modelo y keyframes (hecho).
2. `shutter-render`: evaluación de escena y rasterizador CPU de referencia (hecho).
3. `shutter-render-wgpu`: backend inicial fuera de pantalla (en desarrollo; Normal, formas rellenas y readback RGBA8).
4. Integración de la textura GPU con la aplicación de escritorio eframe/egui (pendiente).
5. Grupos aislados, máscaras y modos de mezcla completos (pendiente).
6. Timeline, medios, audio y exportación MP4 (pendiente).

El backend GPU inicial aún no tiene antialiasing ni soporte para máscaras o modos de mezcla distintos de Normal. El rasterizador CPU sigue siendo la referencia visual.
