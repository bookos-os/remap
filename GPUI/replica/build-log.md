# Verificación de la migración

Fecha: 2026-11-01. La implementación vive íntegramente en `GPUI/`.
Se conserva el código original de Tauri y el servicio `teclasd`.

## Comprobaciones ejecutadas

- Compilación `cargo build --offline --locked` del binario normal.
- Compilación con `--features pruebas-ui` y ejecución real en Wayland/Vulkan.
- `cargo test`: 13 pruebas pasadas; la prueba de contrato requiere bus privado.
- `tests/probar-bus.sh`: esa prueba pasó por separado con motor de compositor
  y con motor de sistema, incluyendo señal Copilot, rechazo, cancelación antes
  de sondear la captura y plazo real de diez segundos.
- `teclasd`: 13 pruebas pasadas con su manifiesto y lock originales.
- Clippy de todos los objetivos con `pruebas-ui` y `-D warnings`, y formato comprobado.
- Regresión de ventana en castellano con compositor simulado y en inglés con
  servicio de sistema simulado: asignar, volver, pulsar/soltar, rechazar, aceptar
  sin confirmar, recuperar, capturar, cancelar por teclado, bloquear diagonales
  verticales, completar atrás y redimensionar al mínimo.
- Comando personalizado guardado por Intro; eliminación confirmada por clic
  y por Tab → Tab → Intro. Los comandos no se ejecutan: los guarda el doble.
- Captura evdev:240, Copilot y F23 aislada: la señal distingue sus identidades.
- Servicio ausente al arrancar y aparecido después: reintento comprobado.
- Servicio que aplica pero no devuelve estado: editor bloqueado hasta releer;
  el reintento recupera la lista que realmente guardó el doble.
- Distribución ANSI elegida desde la interfaz y persistida en `state.json`.
- Interruptor de paleta apagado y encendido, con preferencia persistida y
  colores de la paleta real inspeccionados.
- `desktop-file-validate`, `rpmspec -P`, `bash -n` y sintaxis de los scripts Python.

Las capturas están en [clone-screens](clone-screens/), y las dos referencias
web del original en [screens](screens/). Se han inspeccionado inicio, editor,
comando, confirmación, captura, errores, ANSI, inglés y tamaño mínimo.
La sesión observada usa escala 2 y conserva el origen al redimensionar; las
capturas del tamaño mínimo se encuadran con ese comportamiento medido.

## Límites de la evidencia

Los motores D-Bus de las pruebas son dobles explícitos en buses privados.
No se alteraron remapeos reales, no se capturaron dispositivos con evdev y no
se instaló ni activó un servicio privilegiado. La integración con el hardware,
el LED físico de Bloq Mayús y la instalación del paquete requieren probarlos
en el equipo de destino. La lógica del servicio existente no se modificó.

No se ha generado ni instalado un RPM: su spec y los scripts se validaron,
pero la compilación ejecutada fue de desarrollo. El binario normal no contiene
el control de pruebas. Cargo avisa de incompatibilidad futura de la dependencia
heredada `proc-macro-error2`; no son avisos de Clippy del código de la app.

Las preferencias visuales antiguas estaban dentro del localStorage de WebKit.
GPUI usa el archivo nativo de preferencias de la plantilla. Los remapeos reales
no se ven afectados por ese cambio: siguen siendo propiedad del mismo servicio.
