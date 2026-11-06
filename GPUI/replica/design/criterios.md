# Diseño

Fuente de medidas: `src/style.css`. Fuente de colores: `Tema` de la plantilla,
que lee la paleta compartida `~/.config/bookos/palette.json` al recuperar foco.
El interruptor de integración guarda `bookos.dynamicPalette` en `state.json`,
el equivalente nativo del almacenamiento local de la versión web.

Ventana inicial 1240×740, mínima 1080×620; marco de 36 px. Lateral 384 px con
24 px de margen y tarjetas de 336 px, radio 22. Detalle con 44 px laterales;
tarjeta de teclado radio 25 y relleno 20. Teclas radio 10, seis filas: 34 y
cinco de 48, huecos de 6. Intro ISO único en L, flechas arriba/abajo apiladas.

Catálogos completos de la app original. Los nombres del protocolo no se traducen;
las etiquetas sí. Los iconos de acciones proceden de Heroicons local y los badges
conservan el glifo original. No se usan emojis ni una segunda paleta fija.
