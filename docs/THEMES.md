# Apariencia, estilos y ventana nativa

## Lenguaje visual

curado mezcla dos materiales:

- **Chrome**: sidebar, topbar y footer son una sola superficie continua. Lleva un gradiente suave con brillo del color principal (arriba a la izquierda) y del color del deck B (abajo a la derecha), y encima un grano de dither ordenado. No hay bordes entre sus partes.
- **Sheet**: el contenido vive en una hoja elevada, lisa, con radio, sombra y un filo de luz. La diferencia de material (grano vs liso) y de elevación separa navegación y contenido sin tarjetas ni bordes.

Dentro de la hoja se evita el «todo es una card»: secciones planas, separación por espacio y tipografía, y superficies de otro material solo donde tienen sentido (el mixer usa material chrome, como hardware en el centro de la consola).

Detalles pixel y gradientes suaves conviven: el logo, los keycaps, las etiquetas y el dither son pixel; fondos, botones primarios, platos y barras de progreso usan gradientes del color principal al `--gradient-end` de cada estilo.

## Configuración

Configuración → **Apariencia**:

| Ajuste     | Opciones                                                           | Notas                                                      |
| ---------- | ------------------------------------------------------------------ | ---------------------------------------------------------- |
| Estilo     | curado, pixel, gótico, y2k, mínimo                                 | Materiales, radios, tipografía y textura por defecto       |
| Color base | presets, selector, hex, HSV                                        | Siempre del usuario; cada estilo sugiere uno sin imponerlo |
| Esquema    | oscuro, claro, sistema                                             |                                                            |
| Variación  | neutra, tonal, contraste alto                                      | Tonal tiñe superficies; contraste refuerza texto y filos   |
| Tipografía | del estilo, display, pixel, mono, sistema                          | `auto` usa la del estilo                                   |
| Textura    | del estilo, dither, gradientes suaves, plano                       | `auto` usa la del estilo                                   |
| Waveform   | 3 bandas (graves/medios/agudos), RGB por frecuencia, mono (acento) | Colores por estilo; se adaptan a modo claro                |

Valores iniciales: estilo **curado**, color **#d66154**, oscuro, neutra, tipografía y textura del estilo, waveform de 3 bandas. «Restaurar» vuelve exactamente a esos valores.

### Estilos

| Estilo | Display            | UI            | Textura | Carácter                                            |
| ------ | ------------------ | ------------- | ------- | --------------------------------------------------- |
| curado | Jersey 15          | sistema       | dither  | Cálido, gradiente coral → ámbar, detalles pixel     |
| pixel  | Jersey 15          | Pixelify Sans | dither  | Radio 0, bordes de 2 px, sombras en escalón, sin AA |
| gótico | UnifrakturMaguntia | sistema       | suave   | Negro profundo, viñeta, glow carmesí, filos finos   |
| y2k    | Michroma           | sistema       | suave   | Cromo, brillo diagonal, botones píldora con reflejo |
| mínimo | sistema            | sistema       | plano   | Sin glow ni grano; solo color esencial              |

Etiquetas pequeñas usan Pixelify Sans y los números Geist Mono en todos los estilos salvo tipografía «sistema».

## Implementación

- `src/lib/theme.ts`: matemática RGB/HSV, validación v1→v2, `STYLES` (semillas por estilo), `resolved()` y `palette()` pura. Todo color sale de aquí.
- `src/lib/theme-store.ts`: caché local, preferencia del OS, aplica tokens y atributos `data-style`, `data-typography`, `data-texture`, `data-theme`; genera texturas dither; persiste en SQLite con debounce de 180 ms.
- `src/lib/dither.ts`: matriz Bayer 8×8; produce PNG diminutos (data URL) por color del tema: degradados X/Y, grano y 50 %. CSS los escala con `image-rendering: pixelated`. Solo se regeneran si cambian acento, secundario o texto.
- `src/styles.css`: primer bloque `:root` estructural (fuentes, radios, variables de textura con fallback `none`); segundo bloque generado por `scripts/sync-theme.ts` desde `palette(DEFAULT_THEME)`; después reglas por `data-style`/`data-typography`/`data-texture`.
- `src/components/ThemePanel.tsx`: tarjetas de estilo con su propia paleta, selects de tipografía/textura/waveform, color.
- `src-tauri/src/preferences.rs`: `Theme` v2 con valores por defecto para filas v1; valida enumeraciones y hex.

Reglas:

- Añadir colores como tokens en `palette()`, nunca hex en componentes. Constantes intencionales: presets, semillas de estilo, colores de bandas de waveform.
- Variables estructurales (fuentes, radios, texturas) se declaran en CSS; las de componentes (`--t`, `--p`, `--turn`, `--art-*`, `--zone`) se fijan inline. El test de integración comprueba que cada `var()` exista en una de esas fuentes.
- Las fuentes son locales (Fontsource, OFL). La CSP no permite fuentes remotas. Ver ADR-009.

## Animación e interacción

- Teclas: cada acción ilumina el control asociado (`data-action`) y su keycap (`data-key`) con Web Animations, sin render de React; además un HUD muestra la tecla y la acción («A · Hot cue 2»). Las teclas bloqueadas por modo aparecen como «no disponible».
- Selector de modo con indicador deslizante, play con «respiración», platos que giran según la posición del motor, entrada suave de vistas.
- `prefers-reduced-motion` desactiva animaciones y transiciones.

## Cabecera macOS

`tauri.macos.conf.json` usa `titleBarStyle: Overlay`, `hiddenTitle: true` y controles nativos en (18, 22). La marca «curado» se dibuja a la derecha de los controles, alineada con la topbar; esa zona y la topbar son región de arrastre. No se replica la barra de título en HTML. Windows/Linux conservan decoraciones nativas.

Vista mini: botón en la topbar. Usa `setSize`, `setMinSize` y `setAlwaysOnTop` (permisos acotados en `capabilities/default.json`), recuerda el tamaño anterior y lo restaura al salir.

## Evidencia y límites

- Tests puros: 5 estilos × 10 colores × 3 modos × 3 variantes × OS claro/oscuro. Para chrome y sheet: acento de texto ≥ 4.5:1, texto > 7:1, secundario ≥ 7:1, atenuado > 4.5:1; agudos de la waveform visibles en modo claro; tokens hex válidos; migración v1→v2.
- Contraste matemático no es una auditoría completa. **Ningún estilo nuevo se ha revisado visualmente**: el usuario pidió verificar sin Computer Use ni pruebas visuales. `color-mix()` y unidades de contenedor requieren WebKit de Safari 16+; en macOS 12 con Safari 15 sin actualizar algunos fondos caerían a su valor sin mezcla.
