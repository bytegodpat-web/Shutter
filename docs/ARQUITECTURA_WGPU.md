# Arquitectura del backend de renderizado con wgpu

**Estado:** propuesta de diseño; no es código de producción.  
**Ámbito:** aplicación de escritorio para Windows, macOS y Linux.  
**Principios:** clean-room, render determinista cuando sea posible, backend CPU como referencia y sin dependencia de la interfaz para renderizar.

## 1. Decisiones principales

1. Reutilizar `shutter-render::evaluate(&Composition, time) -> Scene` como frontera entre el modelo y los backends. La GPU no debe volver a interpretar keyframes, jerarquía ni easing.
2. Mantener `shutter-render` como contrato neutral de backend y añadir un crate `shutter-render-wgpu` (nombre provisional) para recursos y comandos GPU. El rasterizador tiny-skia continúa siendo referencia y fallback.
3. Representar la escena evaluada como una lista ordenada de items y añadir una representación de árbol/plan de composición para los grupos que necesitan aislarse en texturas. No aplanar la semántica de grupos antes de resolver máscaras y mezcla.
4. Usar WGSL para los shaders propios y APIs portables de wgpu; evitar extensiones de hardware obligatorias en la primera versión.
5. Renderizar en resolución de composición, no resolución de ventana. La vista previa aplica un transform de cámara/zoom aparte; exportación puede pedir una resolución diferente.
6. Separar la representación visual de la composición de la presentación en pantalla. La misma salida de GPU debe poder copiarse a una textura de preview o leerse para exportación.

## 2. Versión recomendada y compatibilidad

Recomendación inicial: **wgpu 30.x**, con **eframe/egui 0.36.x** si se elige eframe para la aplicación. La documentación publicada lista wgpu 30.0.1 y eframe/egui 0.36.2 con dependencia wgpu 30.0; esta pareja reduce el riesgo de mezclar tipos incompatibles entre versiones principales. Fuentes para confirmar al implementar: [wgpu en docs.rs](https://docs.rs/wgpu/latest/wgpu/) y [eframe en docs.rs](https://docs.rs/eframe/latest/eframe/).

- Fijar versiones compatibles en el workspace y actualizar wgpu y egui como conjunto.
- Verificar MSRV, plataformas y features en los manifiestos exactos antes de añadir dependencias; el MSRV del proyecto puede subir respecto al workspace actual.
- No copiar ejemplos antiguos sin contrastar sus firmas con la versión elegida.
- Las firmas de APIs que pueden cambiar entre versiones se marcan como **(a verificar)** en este documento. No se pretende que los nombres aquí mencionados constituyan código compilable.

## 3. Límites entre crates

### `shutter-core` y `shutter-anim`

Siguen siendo dueños del documento editable y de la evaluación temporal: composición, capas, relaciones padre-hijo, grupos, propiedades y keyframes.

### `shutter-render`

Mantiene:
- el contrato `Scene` y los tipos de item evaluado;
- `evaluate(composition, time)`;
- el rasterizador CPU tiny-skia de referencia;
- tipos neutrales para opciones de render, tamaño, espacio de color y errores si se acuerda introducirlos.

La `Scene` actual puede seguir siendo plana para items ordinarios. Para grupos, máscaras y operaciones de aislamiento se necesita conservar información estructural suficiente. La propuesta es que `evaluate` produzca o pueda producir un **RenderPlan** derivado de la composición: nodos de dibujo, subcomposiciones aisladas y operaciones de máscara. No debe duplicar la lógica de animación; solo describir el orden y las dependencias de renderizado. La forma concreta de extender el contrato se decide antes de implementar.

### Nuevo crate `shutter-render-wgpu`

Responsabilidades:
- inicializar y poseer `Device`, `Queue`, pipelines, buffers, texturas y cachés;
- traducir `Scene`/RenderPlan a comandos GPU;
- gestionar superficies de composición intermedias, máscaras y readback;
- exponer operaciones de render sin depender de widgets de egui.

El crate no debe poseer el documento ni tomar decisiones de UI. El primer prototipo puede vivir dentro de `shutter-render`, pero separar el backend evita que wgpu se convierta en dependencia obligatoria del rasterizador CPU.

### Crate de aplicación

El futuro crate de escritorio conecta modelo, canvas, timeline y paneles. Si se usa eframe, la UI puede emplear `egui` y su integración wgpu, mientras el motor conserva sus propios pipelines. El backend del editor debe poder dibujar a una textura de composición y entregarla al canvas de egui sin convertir toda la imagen a CPU cada frame.

## 4. Pipeline de renderizado

1. **Evaluación temporal:** `evaluate(composition, time)` evalúa transforms, opacidad, relleno, jerarquía y propiedades animadas.
2. **Planificación:** crear un orden de dibujo y detectar límites de aislamiento: grupos, máscaras, modos de mezcla que requieren destino previo, efectos multipaso.
3. **Preparación de geometría:** convertir las formas soportadas a datos de GPU. Mantener un caché de geometría indexado por identidad/revisión de la forma; las transformaciones animadas no deberían volver a teselar la geometría.
4. **Render de formas:** emitir triángulos de paths teselados o geometría de primitivas. La primera entrega puede admitir rectángulos y el conjunto mínimo de formas existente; los paths arbitrarios y strokes complejos se incorporan después.
5. **Composición:** renderizar items en orden estable y respetar la convención de alfa documentada.
6. **Salida:** presentar la textura a la vista previa o copiar el resultado a un buffer de lectura para exportación.

### Vectores en GPU

No asumir que wgpu dibuja paths vectoriales directamente: el hardware rasteriza primitivas. Las formas se convierten a mallas de triángulos en CPU (teselación) y esas mallas se rasterizan en GPU. El diseño debe permitir actualizar solo los vértices cuando cambie la geometría, y actualizar uniforms/buffers pequeños cuando cambie el transform o el color.

- Coordenadas del documento y coordenadas de píxel deben tener una convención única, documentada y probada.
- Usar transformaciones 2D afines en los uniforms o en atributos por instancia.
- Establecer reglas explícitas para antialiasing, bordes, winding, clipping, límites de malla y formas degeneradas.
- El primer objetivo es paridad visual razonable con tiny-skia, no igualdad bit a bit.

## 5. Grupos y texturas intermedias

Los grupos con aislamiento se renderizan en una textura intermedia con formato y resolución definidos. El plan de render debe indicar qué subárbol se renderiza, sus límites en espacio de composición, transform, opacidad y operación de composición posterior.

- Calcular bounds conservadores para el grupo, incluyendo strokes y efectos; añadir padding cuando un filtro pueda extenderse fuera de la forma.
- Mantener un pool de texturas temporales reutilizables por tamaño/formato para reducir asignaciones por frame.
- No usar una textura distinta para cada grupo en cada frame si se puede reutilizar de forma segura.
- Resolver recursión de grupos con una pila de targets y límites de profundidad razonables; reportar errores claros si se exceden recursos.
- La opacidad de grupo se aplica al componer el resultado aislado, no indiscriminadamente a cada hijo, para evitar cambios semánticos en solapamientos.
- Los modos que dependen del color del destino se ejecutan en una pasada de composición que lee la textura acumulada y la textura fuente, o con un método equivalente que respete el orden de capas.

## 6. Máscaras: Mask y Exclude

Las máscaras deben modelarse como una relación explícita entre una capa/grupo fuente y el contenido afectado, no como un simple modo de mezcla de color.

1. Renderizar la fuente de máscara a una textura de máscara/alpha con el mismo sistema de coordenadas y bounds compatibles con el grupo objetivo.
2. Obtener una cobertura normalizada por píxel. En el caso alfa, `Mask` usa la cobertura como factor de visibilidad del objetivo; `Exclude` invierte la cobertura.
3. Aplicar la cobertura al contenido aislado antes de componerlo sobre el destino. La ecuación exacta para alpha premultiplicado y las reglas de múltiples máscaras deben fijarse en pruebas antes de congelar el contrato.
4. Definir si las máscaras siguen transformaciones de grupo, si usan luminancia además de alfa y cómo se comportan cuando la fuente está vacía o deshabilitada. No inferir estas reglas de otra app; documentarlas como semántica propia.
5. Añadir casos para bordes antialias, máscara totalmente negra/blanca, alpha parcial, máscara invertida y máscaras anidadas.

Las máscaras pueden usar R8Unorm para cobertura si basta un canal, pero validar el formato y la compatibilidad del target con las operaciones requeridas **(a verificar)**. Para el primer prototipo, un formato RGBA8Unorm puede simplificar la lectura y depuración.

## 7. Modos de mezcla

Shutter ya expone 28 modos de mezcla en el modelo. Mantener sus nombres/semántica en una capa neutral, sin codificar lógica de negocio en la UI.

- Clasificar modos por estrategia: mezcla fija de hardware cuando la operación coincide exactamente; shader de composición cuando requiere leer el destino; y modo no soportado explícitamente mientras se implementa.
- No asumir que los blend states fijos de wgpu cubren todos los modos de edición, ni que pueden implementar fórmulas que requieren muestrear el destino.
- Para composición por shader, usar una textura de destino previa y una textura fuente, escribir a un tercer target o usar ping-pong para evitar leer y escribir simultáneamente la misma textura.
- Definir fórmula por modo y probarla con colores conocidos, alpha parcial y capas superpuestas.
- Especificar si el cálculo ocurre en espacio lineal o sRGB; conversiones implícitas producen diferencias difíciles de depurar.
- Documentar la representación de alpha (preferiblemente premultiplicado durante composición) y convertir en los bordes de entrada/salida si es necesario.

La implementación inicial puede cubrir Normal y algunos modos simples, pero debe mostrar un resultado claro para modos pendientes; nunca sustituir silenciosamente un modo por Normal.

## 8. Lectura de píxeles y exportación

La GPU no debe bloquear la UI esperando una lectura por cada frame de preview. Para exportación, el renderizador recibe una solicitud de frame y entrega pixels a un consumidor de exportación.

- Crear un buffer de lectura con `MAP_READ` y `COPY_DST`; copiar la textura de salida a un buffer usando el layout de filas alineado requerido por wgpu **(a verificar)**.
- Mapear el buffer después de que termine el trabajo GPU, copiar los bytes a memoria propiedad del consumidor y desmapearlo antes de reutilizar el buffer.
- Respetar bytes por fila alineados, tamaño de textura, formato de salida y orientación vertical. No asumir que el stride coincide con ancho × 4.
- Usar un pequeño pool de buffers para permitir que la GPU trabaje mientras la CPU consume un frame anterior.
- En exportación, permitir renderizado fuera de pantalla sin ventana/surface. Una pérdida de surface no debe impedir generar un archivo.
- Hacer el readback por lotes/asíncrono cuando sea posible; la forma exacta de `map_async`, el polling de dispositivo y el callback depende de la versión elegida **(a verificar)**.

El contrato del exportador debe definir claramente RGBA/BGRA, alpha, espacio de color, stride, timestamp y propiedad del buffer. El empaquetado del video es otra capa; el backend GPU produce frames, no MP4.

## 9. Integración con egui/eframe

Recomendación: mantener el render de la composición independiente de egui y presentar su textura en el canvas. `eframe` ya integra `egui` con wgpu en la versión recomendada, pero el acceso al `Device`/ `Queue` compartidos y el registro de una textura externa dependen de la API de eframe elegida **(a verificar)**.

- Evitar crear una segunda instancia de GPU/surface solo para el motor si la integración puede compartir de forma segura el dispositivo y la cola.
- Si compartir dispositivo complica la primera versión, empezar con un camino de integración mínimo y medir la copia adicional antes de optimizar.
- No ejecutar render pesado dentro del código de layout de egui. Separar actualización del documento, evaluación de escena y trabajo GPU.
- La UI solicita invalidaciones: cambio de frame, transform, zoom, tamaño de canvas o propiedad; el motor reutiliza cachés cuando la escena no cambió.
- Manejar resize, minimización, pérdida/reconfiguración de surface, falta de adaptador y fallback de CPU.
- El canvas transforma el resultado al zoom/pan de vista previa; estos controles no deben cambiar las dimensiones del proyecto ni la salida de exportación.

## 10. Pruebas: GPU frente a CPU

Tiny-skia es la referencia inicial. Comparar renderizados de una misma escena en CPU y GPU con dimensiones, escala, tiempo y espacio de color idénticos.

### Niveles de pruebas

1. **Unitarias sin GPU:** evaluación de composición, transformaciones, orden de capas, clasificación de modos y construcción de RenderPlan.
2. **Pruebas de GPU condicionadas por entorno:** crear un adaptador real cuando exista; permitir un backend/software de CI si está disponible. Saltar con motivo explícito si no hay adaptador, no fallar como si fuera regresión del renderer.
3. **Golden images:** escenas pequeñas con formas, transform, opacidad, grupos, máscaras y modos de mezcla.
4. **Comparación numérica:** calcular error absoluto medio, máximo y porcentaje de píxeles fuera de tolerancia. Usar tolerancias configurables por backend debido a diferencias de antialiasing y precisión.
5. **Pruebas de recursos:** resize repetido, texturas liberadas, muchos items, composición vacía, bounds extremos, pérdida del dispositivo y readback de varios frames.
6. **Pruebas de determinismo del export:** mismas entradas deben producir timestamps y dimensiones idénticos; comparar pixels con tolerancia, no exigir bit-identidad entre GPU distintas.

Las pruebas deben reportar imágenes de referencia y resultado cuando sea posible. Una diferencia visual no debe corregirse elevando tolerancias sin diagnosticar si procede de alpha, orden de capas, espacio de color, coordenadas o rasterización.

## 11. Rendimiento, memoria y errores

- Reutilizar pipelines, bind groups, buffers de vértices y targets.
- Evitar una asignación GPU por capa y frame; agrupar geometría compatible cuando no rompa el orden de composición.
- Instrumentar CPU evaluation, tessellation, render passes, readback y encoding por separado.
- Poner límites a resolución, tamaño de textura, profundidad de grupos y cantidad de memoria temporal; devolver errores recuperables.
- Eliminar o invalidar cachés al cambiar geometría, tamaño, formato o dispositivo.
- Usar el backend CPU como fallback explícito y diagnóstico, no como sustitución silenciosa de funciones GPU faltantes.

## 12. Riesgos y decisiones pendientes

- **Semántica de grupos y máscaras:** fijar reglas con pruebas antes de implementar el backend.
- **Teselación de paths:** elegir o desarrollar una solución con licencia compatible y documentar sus límites; no reutilizar código o assets de aplicaciones de referencia.
- **Blend modes:** definir fórmulas y espacios de color para los 28 modos.
- **Compatibilidad API:** verificar las APIs marcadas **(a verificar)** con wgpu 30.x y eframe 0.36.x en un spike compilable.
- **Licencias y códecs:** mantener clean-room. Para H.264/H.265 usar codificadores del sistema o FFmpeg enlazado dinámicamente; no incorporar ni enlazar x264. Revisar licencias y distribución de FFmpeg por plataforma.
- **Alcance de la primera entrega:** formas básicas, Normal blend, grupo aislado, alpha-mask, readback y comparación CPU/GPU; después ampliar máscaras y modos de mezcla.

## 13. Criterio de salida del backend inicial

El backend inicial se considera listo cuando puede renderizar una escena animada existente a una textura GPU, mostrarla en el canvas de escritorio, leer el frame de vuelta con stride correcto y pasar una suite de comparación CPU/GPU para las formas y operaciones que declara soportar. Las operaciones pendientes deben estar documentadas y no cambiar silenciosamente de semántica.
