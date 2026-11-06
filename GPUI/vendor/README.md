Adaptación local del backend Linux y del selector de plataforma de la copia
Glass-HQ/gpui usada por `plantilla-gpui` (650aab5, con parches de BookOS).
Los demás crates, incluido `gpui`, siguen siendo los compartidos de `fonts/vendor/gpui`.
No se descarga otra versión ni se modifica la plantilla compartida.

Diferencias de código:
- `gpui_linux/src/linux/platform.rs`: conserva `keycode - 8` como código evdev.
- `gpui_linux/src/linux/wayland/client.rs`: entrega también pulsación y suelta
  de modificadores. El estado agregado no distingue Ctrl izquierdo del derecho.

El manifiesto conserva las dependencias y licencias originales (Apache-2.0),
resolviendo las rutas compartidas desde esta carpeta. `gpui_platform` solo
cambia de ubicación para seleccionar este backend; su código no se modifica.
