# FPS personalizados en Shutter

## Estado actual

El modelo de composición guarda la tasa de fotogramas como un entero sin una lista cerrada de presets. `Composition::new(...)` normaliza únicamente los valores inferiores a 1 a 1 FPS; no impone un máximo de 60 ni de 120 FPS. El valor forma parte del proyecto serializado y debe sobrevivir al ciclo guardar/abrir.

Esto permite experimentar con tasas inusuales, incluso valores muy altos, sin tener que editar XML ni depender de una lista de opciones de interfaz.

## FPS de composición frente a velocidad de render

El FPS indica cuántos fotogramas corresponden a cada segundo de la línea de tiempo; no garantiza que el renderizador produzca esa cantidad de imágenes por segundo en tiempo real. Un proyecto de 300.000 FPS de un segundo implicaría 300.000 fotogramas si se exportara cada instante.

Por eso, la interfaz futura debe aceptar FPS personalizados, pero advertir sobre la cantidad estimada de fotogramas y el coste de exportación para valores extremos. No debe truncar silenciosamente un valor personalizado a 60 o 120.

## Compatibilidad de exportación

La exportación de vídeo todavía no está implementada. Al construirla, Shutter deberá validar por separado los límites del codificador y del contenedor seleccionados. Si una combinación no admite la tasa solicitada, debe mostrar un error explícito y no cambiarla silenciosamente. La validación de exportación no debe impedir guardar el proyecto con su FPS personalizado.

## Prueba de regresión

`custom_frame_rates_are_preserved_without_a_preset_limit` verifica que 300.000 FPS se conservan tanto al crear una composición como al serializar y deserializar un proyecto.
