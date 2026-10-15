# Plan de hitos: de wgpu a MP4

**Estado:** plan propuesto. Los hitos son secuenciales salvo que se indique lo contrario. Cada tarea debe terminar con pruebas y una descripción verificable; no se considera terminada solo porque compile en una máquina.

## Reglas de trabajo

- Aplicación exclusivamente de escritorio: Windows, macOS y Linux. No diseñar un cliente móvil.
- Mantener `shutter-render` CPU/tiny-skia como referencia.
- Clean-room: implementar comportamiento y fórmulas genéricas desde especificaciones propias; no copiar código, iconos, presets, plantillas, nombres propios ni otros assets de aplicaciones existentes.
- H.264/H.265 se codifican con codificadores del sistema o FFmpeg enlazado dinámicamente. No usar ni enlazar x264.
- Para cada cambio ejecutar, según proceda, `cargo fmt --all`, `cargo test --workspace` y `cargo clippy --workspace --all-targets -- -D warnings`.
- Cada hito termina con un commit pequeño o PR, pruebas repetibles y una nota de limitaciones conocidas.

## Hito 0 — Contratos y línea base

### 0.1 Registrar el estado actual
- [ ] Documentar crates, comandos de prueba, ejemplo de render y limitaciones conocidas.
- [ ] Guardar una escena de referencia estable y sus PNG generados por tiny-skia.
- **Terminado cuando:** una persona nueva puede ejecutar las pruebas actuales y generar la imagen de referencia siguiendo solo el README.

### 0.2 Fijar semántica visual
- [ ] Especificar orden de capas, coordenadas, transform afín, opacidad, alpha, espacio de color y significado de cada modo de mezcla existente.
- [ ] Definir qué significa un grupo aislado y qué hacen Mask y Exclude, incluidos casos vacíos y alpha parcial.
- **Terminado cuando:** las reglas tienen ejemplos de entrada/salida y tests CPU para los casos críticos.

### 0.3 Seleccionar versiones compatibles
- [ ] Probar un spike mínimo con wgpu 30.x y eframe/egui 0.36.x; confirmar MSRV, features y targets de escritorio.
- [ ] Anotar firmas de APIs y compatibilidad que se hayan verificado.
- **Terminado cuando:** el spike compila en CI en al menos Linux y un runner de Windows o macOS, y las versiones están fijadas en el workspace.

## Hito 1 — Esqueleto del backend GPU

### 1.1 Crear el crate de backend
- [ ] Añadir `shutter-render-wgpu` (o justificar mantenerlo dentro de `shutter-render`).
- [ ] Definir errores recuperables: no hay adaptador, dispositivo perdido, formato no soportado, límites excedidos y fallo de shader.
- **Terminado cuando:** el workspace compila sin obligar al crate CPU a depender de wgpu si se eligió separación.

### 1.2 Inicializar dispositivo y cola
- [ ] Crear instancia, enumerar adaptadores y solicitar dispositivo con límites conservadores.
- [ ] Registrar nombre del adaptador, backend y límites útiles para diagnóstico.
- [ ] Definir estrategia de fallback cuando no hay GPU adecuada.
- **Terminado cuando:** una prueba o ejemplo informa adaptador/backend y termina limpiamente si no hay hardware disponible.

### 1.3 Primera textura y pipeline
- [ ] Crear una textura RGBA para composición, una vista y un pipeline WGSL de borrado/color sólido.
- [ ] Renderizar un color conocido fuera de pantalla.
- **Terminado cuando:** el readback de la textura coincide con el color esperado dentro de la tolerancia definida.

## Hito 2 — Formas y transformación en GPU

### 2.1 Contrato de geometría
- [ ] Definir una representación GPU-neutral para rectángulos y primitivas ya soportadas por `Shape`.
- [ ] Documentar unidades, origen, winding, bounds y tratamiento de degenerados.
- **Terminado cuando:** cada primitiva soportada tiene fixtures geométricos y límites explícitos.

### 2.2 Teselación inicial
- [ ] Generar mallas de triángulos para las formas básicas en CPU.
- [ ] Mantener un caché de malla por identidad/revisión de geometría.
- **Terminado cuando:** las formas básicas se ven correctamente sin depender del rasterizador CPU para dibujar sus píxeles.

### 2.3 Uniforms y transforms
- [ ] Aplicar el transform afín de `Scene` sin recalcular vértices por cada cambio de posición/rotación/escala.
- [ ] Verificar aspect ratio, origen, clipping y escalas no uniformes.
- **Terminado cuando:** una batería de transforms coincide con tiny-skia dentro de la tolerancia acordada.

### 2.4 Relleno, opacidad y orden
- [ ] Implementar color de relleno, alpha y orden estable de items.
- [ ] Definir alpha premultiplicado o straight alpha y usar la convención de forma coherente.
- **Terminado cuando:** escenas de dos y varias capas, con opacidades diferentes, pasan pruebas de pixels.

### 2.5 Antialiasing de formas
- [ ] Medir los bordes de referencia y elegir una estrategia inicial: geometría de cobertura, multisampling cuando sea compatible o técnica propia.
- [ ] Documentar diferencias frente a tiny-skia.
- **Terminado cuando:** los bordes no presentan huecos visibles en fixtures y el error se mantiene dentro de la tolerancia fijada.

## Hito 3 — Comparación y estabilidad GPU/CPU

### 3.1 Harness de imágenes
- [ ] Crear pruebas que rendericen la misma composición con CPU y GPU.
- [ ] Calcular MAE, error máximo y porcentaje de píxeles fuera de tolerancia.
- [ ] Guardar imágenes diff como artefactos de CI cuando la comparación falle.
- **Terminado cuando:** una diferencia intencional produce un fallo legible con métricas y archivos de diagnóstico.

### 3.2 Suite mínima de escenas
- [ ] Incluir formas básicas, transforms, opacidad, superposición, bordes, escena vacía y varios tamaños.
- [ ] Cubrir al menos dos escalas de salida.
- **Terminado cuando:** la suite es repetible y no depende de la resolución de la ventana.

### 3.3 Ciclo de vida y errores
- [ ] Probar resize, recreación de targets, falta de adaptador, errores de shader y dispositivo perdido cuando el entorno lo permita.
- [ ] Permitir tests saltados explícitamente cuando no hay adaptador, con razón registrada.
- **Terminado cuando:** no hay panic por fallos recuperables y la CI distingue fallo real de falta de hardware.

## Hito 4 — Integración con eframe/egui

### 4.1 Aplicación de escritorio mínima
- [ ] Crear el crate de aplicación con eframe, una ventana y un canvas.
- [ ] Confirmar targets nativos y documentar dependencias del sistema para Linux.
- **Terminado cuando:** la aplicación abre en los sistemas de CI previstos y muestra un canvas vacío.

### 4.2 Presentar textura del motor
- [ ] Mostrar el resultado del backend en el canvas de egui.
- [ ] Resolver el ownership y ciclo de vida de la textura compartida o la ruta de copia elegida.
- **Terminado cuando:** una escena animada se ve en el canvas y el resize no deja texturas obsoletas.

### 4.3 Interacción de vista
- [ ] Implementar zoom y pan del canvas sin cambiar las dimensiones del documento.
- [ ] Traducir coordenadas de mouse a coordenadas de composición.
- **Terminado cuando:** un punto conocido del documento sigue alineado tras zoom, pan y resize.

### 4.4 Invalidación y rendimiento
- [ ] Separar UI, evaluación de escena y render GPU.
- [ ] Re-renderizar solo cuando cambien tiempo, documento, tamaño o parámetros visuales.
- [ ] Medir CPU/GPU time en una escena de referencia.
- **Terminado cuando:** la vista estática no hace trabajo de render completo innecesario y la medición queda registrada.

## Hito 5 — Grupos, aislamiento y máscaras

### 5.1 RenderPlan jerárquico
- [ ] Preservar los límites de grupos necesarios para operaciones aisladas.
- [ ] Producir un plan ordenado que no duplique la evaluación de animación.
- **Terminado cuando:** escenas equivalentes con y sin grupos simples producen el resultado documentado.

### 5.2 Pool de texturas intermedias
- [ ] Asignar targets reutilizables según bounds, tamaño y formato.
- [ ] Gestionar padding, resize y liberación de recursos.
- **Terminado cuando:** el uso de memoria se mantiene estable al reproducir una escena con grupos repetidamente.

### 5.3 Opacidad de grupo
- [ ] Renderizar hijos a una superficie aislada y aplicar opacidad al componer el grupo.
- **Terminado cuando:** los hijos solapados se comportan como una unidad y pasan los fixtures CPU/GPU.

### 5.4 Máscara alfa Mask
- [ ] Renderizar fuente de máscara a textura de cobertura.
- [ ] Aplicar la cobertura al grupo objetivo antes de componerlo.
- **Terminado cuando:** máscaras blanca, negra, parcial y antialias producen los resultados especificados.

### 5.5 Máscara invertida Exclude
- [ ] Implementar la cobertura complementaria y definir interacción con múltiples máscaras.
- **Terminado cuando:** fixtures de exclusión, máscaras anidadas y fuente vacía pasan la suite.

### 5.6 Máscaras transformadas y bounds
- [ ] Cubrir transforms de grupo, clipping, límites con padding y resolución diferente.
- **Terminado cuando:** no hay halos ni recortes incorrectos en casos de borde definidos.

## Hito 6 — Modos de mezcla

### 6.1 Catálogo de fórmulas
- [ ] Clasificar los 28 modos entre blend state fijo, shader de composición y pendiente.
- [ ] Documentar fórmulas, espacio de color y tratamiento de alpha para cada modo.
- **Terminado cuando:** cada modo tiene especificación y casos de prueba de colores conocidos.

### 6.2 Pipeline de composición por shader
- [ ] Añadir lectura de fuente y destino mediante texturas distintas y ping-pong seguro.
- [ ] Implementar primero Normal y modos simples seleccionados.
- **Terminado cuando:** los modos soportados pasan fixtures y los no soportados muestran estado explícito.

### 6.3 Completar los 28 modos
- [ ] Implementar los modos restantes por grupos, con un commit por familia.
- [ ] Comparar cada modo con la referencia CPU y corregir divergencias semánticas.
- **Terminado cuando:** los 28 modos tienen tests de fórmula y tests de composición; ninguno cae silenciosamente a Normal.

### 6.4 Rendimiento de composición
- [ ] Reducir cambios de pipeline y asignaciones sin alterar el orden de dibujo.
- [ ] Añadir benchmark de muchas capas y muchos grupos.
- **Terminado cuando:** se publica una línea base de tiempo y memoria para escenas de referencia.

## Hito 7 — Lectura de frames y render fuera de pantalla

### 7.1 Buffer de readback
- [ ] Copiar textura a buffer compatible con lectura CPU.
- [ ] Respetar alineación de filas y bytes por píxel.
- **Terminado cuando:** imágenes de varios anchos, incluidos anchos no alineados, se leen sin corrupción.

### 7.2 Lectura asíncrona y pool
- [ ] Evitar bloquear la UI por cada frame; reutilizar buffers después de completar la lectura.
- [ ] Definir ownership y cancelación si se cierra el proyecto/exportación.
- **Terminado cuando:** varios frames se procesan sin sobrescribir buffers en uso ni filtrar recursos.

### 7.3 Render headless
- [ ] Renderizar sin ventana ni surface de presentación.
- [ ] Permitir resolución de exportación independiente de la vista.
- **Terminado cuando:** un comando de prueba genera una secuencia PNG sin abrir la UI.

### 7.4 Contrato de frame
- [ ] Definir dimensiones, stride, formato, alpha, espacio de color, índice de frame y timestamp.
- **Terminado cuando:** el consumidor recibe frames correctos y los tests detectan cambios de orientación/formato.

## Hito 8 — Timeline y control temporal mínimo

### 8.1 Modelo de tiempo de proyecto
- [ ] Definir FPS como racional (numerador/denominador), duración, rango de trabajo y convención de timestamps.
- [ ] Evitar acumular error de coma flotante avanzando frame por frame.
- **Terminado cuando:** el frame N siempre corresponde al timestamp racional documentado.

### 8.2 Reproducción y scrubbing
- [ ] Conectar el control temporal de la UI a `evaluate`.
- [ ] Añadir play/pause, salto a frame y scrubbing.
- **Terminado cuando:** el canvas muestra el frame esperado y los keyframes mantienen sus tiempos durante scrubbing repetido.

### 8.3 Timeline inicial
- [ ] Mostrar capas y keyframes existentes; selección y zoom horizontal básicos.
- **Terminado cuando:** se puede seleccionar una capa y saltar a uno de sus keyframes sin editar datos manualmente.

## Hito 9 — Entrada de medios y secuencia de imágenes

### 9.1 Contrato de medios
- [ ] Definir una abstracción para imágenes fuente, duración, dimensiones, color y errores de decodificación.
- **Terminado cuando:** una imagen se carga y su ciclo de vida está cubierto por pruebas.

### 9.2 Importar secuencia
- [ ] Importar PNG como secuencia o frames de prueba, con orden determinista.
- [ ] Definir qué ocurre ante archivos faltantes y dimensiones mixtas.
- **Terminado cuando:** una secuencia de imágenes se reproduce en los timestamps correctos.

### 9.3 Audio mínimo
- [ ] Definir sample rate, canales, timeline de audio y sincronización con el reloj del proyecto.
- [ ] Decidir una librería o backend de audio compatible con escritorio y su licencia.
- **Terminado cuando:** un clip de audio puede colocarse en la línea de tiempo y la duración/rango son correctos; reproducción en tiempo real puede quedar para un hito separado.

## Hito 10 — Codificación y muxing de vídeo

### 10.1 Seleccionar estrategia por plataforma
- [ ] Definir adaptadores para codificadores del sistema y/o FFmpeg enlazado dinámicamente.
- [ ] Verificar distribución, licencias y disponibilidad de codecs en cada plataforma.
- [ ] Excluir explícitamente x264 de dependencias, builds y documentación.
- **Terminado cuando:** la estrategia por plataforma está documentada y CI comprueba que no se introduce una dependencia x264.

### 10.2 Probar encoder aislado
- [ ] Enviar una secuencia sintética de frames a un encoder H.264 disponible.
- [ ] Definir formato de píxel, conversión de color, timestamps, bitrate/calidad y cierre/finalización.
- **Terminado cuando:** se produce un flujo de vídeo decodificable y se verifican dimensiones y cantidad de frames.

### 10.3 Contenedor MP4
- [ ] Añadir muxing MP4 mediante la ruta de FFmpeg enlazado dinámicamente o una solución compatible elegida.
- [ ] Manejar errores de biblioteca ausente, codec no disponible, disco lleno y cancelación.
- **Terminado cuando:** el archivo se abre en al menos dos reproductores/decodificadores independientes y reporta duración, FPS y dimensiones esperadas.

### 10.4 Exportador de frames a vídeo
- [ ] Conectar render headless, readback, conversión de píxeles y encoder con backpressure.
- [ ] Evitar almacenar todos los frames en RAM; usar una cola acotada.
- **Terminado cuando:** un proyecto corto exporta MP4 sin crecimiento ilimitado de memoria y el número de frames coincide con el rango.

### 10.5 Audio y sincronización
- [ ] Mezclar o pasar audio al muxer según la arquitectura elegida.
- [ ] Definir sample-accurate timestamps, duración, silencio y desajustes entre vídeo/audio.
- **Terminado cuando:** un clip audiovisual de prueba se reproduce sincronizado y el archivo final tiene la duración esperada.

### 10.6 Exportación robusta
- [ ] Barra de progreso, cancelación, limpieza de archivos temporales y errores accionables.
- [ ] Añadir exportaciones de prueba para resolución pequeña y una duración mayor.
- **Terminado cuando:** cancelar o fallar no deja archivos finales corruptos presentados como válidos.

## Hito 11 — Calidad de lanzamiento

### 11.1 Compatibilidad de escritorio
- [ ] Probar Windows, macOS y Linux en hardware o runners disponibles; documentar mínimos y fallback CPU.
- **Terminado cuando:** los caminos soportados están en CI o en una matriz manual reproducible con resultados registrados.

### 11.2 Calidad y rendimiento
- [ ] Ejecutar tests, clippy, formato, comparación CPU/GPU y benchmarks.
- [ ] Perfilar una composición con muchas capas y una exportación larga.
- **Terminado cuando:** no hay errores críticos conocidos, los límites de memoria están documentados y el rendimiento de referencia queda guardado.

### 11.3 Clean-room y licencias
- [ ] Revisar dependencias, licencias de teselación, UI, media y FFmpeg.
- [ ] Confirmar que no se han añadido assets o código copiados y que no se enlaza x264.
- **Terminado cuando:** el inventario de dependencias/licencias y la nota clean-room se incluyen en el repositorio.

### 11.4 Documentación de usuario
- [ ] Documentar build, requisitos por plataforma, formatos, codecs disponibles, limitaciones de GPU y exportación.
- **Terminado cuando:** una persona limpia puede construir, abrir un proyecto de ejemplo y exportar un MP4 siguiendo las instrucciones.

## Orden sugerido para el primer ciclo

Priorizar **0 → 1 → 2 → 3 → 4 → 5 → 6 → 7**. No empezar con la exportación MP4 hasta que la lectura headless de frames sea estable. Los hitos 8 y 9 pueden desarrollarse en paralelo con partes del backend una vez que los contratos de tiempo y frame estén definidos; los hitos 10 y 11 dependen de esos contratos.

## Definición global de terminado

Un hito solo se cierra si:
- la funcionalidad está documentada y cubierta por tests relevantes;
- las pruebas existentes siguen pasando;
- las limitaciones y los casos no soportados son explícitos;
- no se han incorporado código o assets ajenos al enfoque clean-room;
- el trabajo puede revisarse en un commit o PR pequeño y reproducible.
