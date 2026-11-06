# Arquitectura

Aplicación Rust nativa GPUI en `GPUI/`. `bookos-ui` se consume por ruta desde la
plantilla. Un ejecutor Tokio de un hilo atiende D-Bus sin bloquear la ventana.
Se conservan los clientes y las leyendas de la versión Tauri, retirando sus macros.
No se escribe `teclas.conf`: lo valida y persiste el compositor o el servicio.

Las peticiones se envían de una en una, conservando la última lista pedida.
Cada respuesta se comprueba mediante `GetKeyRemaps`; una discrepancia se reintenta
hasta tres veces. La ausencia de motor se vuelve a consultar al reintentar.

La copia local de `gpui_linux` conserva códigos evdev y eventos de modificadores;
la de `gpui_platform` la selecciona. El tipo GPUI sigue siendo el compartido de la
plantilla. No se modifica ningún archivo fuera de `GPUI/`.

El marco deriva del de la plantilla, con botones de 28×28, separación de 4 px,
margen derecho de 6 px y glifos de 16 px para cumplir la medida del HIG.
