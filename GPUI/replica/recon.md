# Reconocimiento de Remap

Fecha: 2026-11-01. Fuente: árbol local de Remap, versión 0.1.0.

| Fuente | Evidencia |
|---|---|
| `../../src/main.js` | Pantallas, teclado físico, editor, captura, restricciones por motor y preferencias |
| `../../src/style.css` | Lateral 384 px, tarjetas 336 px, teclado de 60 columnas y seis filas |
| `../../src/locales.js`, `../../src/i18n.js` | Catálogo completo ES/EN y fallback español |
| `../../src-tauri/src/escritorio.rs` | Métodos y señales de ambos motores D-Bus |
| `../../src-tauri/src/leyendas.rs` | Leyendas XKB de tres niveles, teclas muertas y variantes |
| `../../../plantilla-gpui/README.md` | Componentes comunes y dependencias GPUI compartidas |
| `../../../BookOS-HIG/AI-DESIGN-SYSTEM.md` | Tokens, paleta, ventanas, estados e idioma |

Observado en código: S01 inicio con remapeos, preferencias y ayuda; S02 editor de
una tecla y selector agrupado; S03 campo de comando; D01 captura; D02 confirmación
de eliminación. El teclado permanece a la derecha en todos los estados.

Flujos: F01 seleccionar tecla → asignar → confirmar por lectura; F02 comando →
guardar; F03 detectar tecla → señal o plazo de diez segundos; F04 quitar → confirmar;
F05 cambiar distribución visual → persistir; F06 probar pulsación/suelta y Bloq Mayús.

Modelo conocido por código: lista JSON de `{origen, destino}`. Los nombres y códigos
evdev forman parte del contrato del compositor. Los datos persistentes de remapeo
siguen siendo propiedad del servicio; GPUI solo guarda preferencias de su interfaz.

No se infiere un servicio nuevo. Se conserva `bookos-teclasd` sin modificarlo.
No hay pagos, cuentas, red ni contenido externo que reconstruir.

Cambios requeridos respecto a la interfaz vieja: idioma exclusivamente automático
(`LC_ALL` → `LC_MESSAGES` → `LANG`, con fallback español); interruptor propio de
paleta dinámica exigido por el HIG. El resto mantiene estructura y funciones.

Las referencias del original se obtienen del HTML local servido sin modificarlo,
con doble explícito de la frontera Tauri y leyendas de libxkbcommon. No prueban el
backend Tauri ni el acceso a dispositivos. Las pruebas GPUI usan D-Bus privado.
