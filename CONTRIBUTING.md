# Cómo contribuir

## Regla clean-room

Shutter recrea *funciones* que se han observado en otras aplicaciones de edición. Para evitar problemas
legales y de autoría:

- **No** copies código fuente, aunque sea público o esté filtrado, de ninguna app de referencia.
- **No** uses sus iconos, logotipos, presets, plantillas, tipografías, audio ni ningún otro asset.
- **No** uses sus nombres propios para efectos, funciones o el propio proyecto. Los nombres genéricos
  de la industria (blur, glow, keyframe, blend mode…) sí se pueden usar.
- La interfaz puede parecerse en *distribución*, pero el arte visual es propio.
- Si un efecto es de un autor externo, no lo repliques: haz un equivalente genérico propio.

Describe en el PR de dónde sale cada idea ("comportamiento observado", "fórmula estándar", etc.).

## Códecs

H.264 y H.265 tienen patentes. Preferimos los codificadores del sistema (VideoToolbox, Media
Foundation, VA-API, NVENC) o FFmpeg enlazado dinámicamente. No se incluye `x264` (GPL) en el proyecto.

## Antes de abrir un PR

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Toda funcionalidad nueva lleva pruebas. Los commits siguen un tono descriptivo y breve.

## Licencia de las contribuciones

Al contribuir aceptas que tu aportación se distribuya bajo la licencia Apache 2.0 del proyecto.
