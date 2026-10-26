# Estado de curado

Actualizado: **2026-10-25** (America/Hermosillo). Versión **0.1.0** con rediseño visual, waveforms de 3 bandas y cola con prefetch. Base funcional de pruebas locales; no se ha completado la matriz de producción de QUALITY.

## Para retomar

1. Leer este documento, ARCHITECTURE, ROADMAP y [SESSIONS](SESSIONS.md).
2. La preferencia vigente es **sin Computer Use ni pruebas visuales**. Usar código, compilación y pruebas automatizadas. **El rediseño de esta sesión no se ha visto en pantalla**: pedir al usuario una revisión visual o autorización explícita antes de iterar sobre detalles de estilo.
3. Nombre siempre en minúsculas: **curado**. Color inicial exacto **#d66154**, estilo **curado**.
4. Siguiente prioridad estructural: S1 streaming con presupuesto de memoria. Siguiente prioridad de experiencia: S2 reloj de posición (playhead/platos a 60 FPS sin violar el invariante 2).

## Implementado

| Área         | Estado y alcance                                                                                                                        |
| ------------ | --------------------------------------------------------------------------------------------------------------------------------------- |
| Stack        | Bun + React/TS/Vite + Tauri 2; crate de audio Rust independiente                                                                        |
| Reproducción | CPAL, Symphonia, mono/estéreo, 2 decks y 4 one shots, seek, fin de pista                                                                |
| Mezcla       | Equal power, master, EQ básico, LP, echo, varispeed, nudge, loops IN/OUT                                                                |
| Waveforms    | Análisis offline 3 bandas a 50 bins/s en la decodificación, BLOB SQLite v3, IPC binario, vista general + detallada (8 s)                |
| Cues         | Tres por track, estado atómico compartido entre decks, autosave SQLite y restauración al cargar                                         |
| Biblioteca   | Archivos/carpetas, metadatos, búsqueda, favoritos, playlists; listado sin peaks (escala con bibliotecas grandes)                        |
| Cola         | Worker Rust, avance nativo, repeat off/one/all; **prefetch** 15 s antes del final; repeat-one sin redecodificar; sin gapless            |
| Modos        | Player/mix persistentes, política nativa compartida por UI/teclado/MIDI                                                                 |
| Entrada      | Atajos físicos remapeables con **feedback visual** (control, keycap y HUD), mapa de teclado clicable, MIDI Learn (EQ unitario en CC 64) |
| UI           | Chrome continuo + sheet elevada; consola A · mixer · B con platos, perillas, faders verticales y medidores LED en dB                    |
| Vistas       | Completa y **mini siempre encima** (460×132) con restauración de tamaño                                                                 |
| Apariencia   | Estilos curado/pixel/gótico/y2k/mínimo, claro/oscuro/sistema, variaciones, tipografía, textura (dither/suave/plano), waveform           |
| Marca        | Nombre en minúsculas; logo pixel (anillo «c» + eje) en UI, SVG y set de iconos regenerado por geometría                                 |
| OS           | Tray/menu bar, cerrar para ocultar si hay tray, recuperar dispositivo/buffer manualmente                                                |
| Datos        | SQLite WAL, esquema v3, migraciones v1→v2→v3 probadas, IDs estables, originales conservados                                             |

## Evidencia automatizada (esta sesión)

- `bun run check`: **15 tests TypeScript** en 4 archivos, 11.404 assertions: paleta en 5 estilos × 10 colores × modos × variantes con contraste sobre chrome y sheet, migración de tema v1→v2, tokens/variables CSS, permisos de ventana exactos, política de modo del teclado, formato binario de waveform, matriz Bayer y geometría del logo.
- `cargo test --workspace`: **19 tests Rust** (10 motor + 9 app): allocator sin alloc/dealloc en render, análisis de bandas (60 Hz → graves, 9 kHz → agudos, 50 bins/s a 44.1 y 48 kHz), migración v1→v3 con backfill sin sobrescritura, prefetch de cola, repeat-one por seek, tema v1→v2 en Rust.
- `cargo clippy --workspace --all-targets -- -D warnings`: limpio. `cargo fmt --all -- --check` y `bun run format:check`: limpios.
- `bun run build`: tipos y frontend de producción (JS 318 KB, 101 KB gzip; CSS 48 KB; fuentes woff2 ~120 KB).
- `bun run desktop:build`: `target/release/bundle/macos/curado.app` ARM64 de **13,09 MiB**, firma ad hoc y `codesign --verify --deep --strict` correctos. `CFBundleName` = curado, identificador sin cambios.
- Probe offline (render sin cambios): P50 **16,75 µs**, P95 **17,38 µs**, P99 **18,12 µs**, máximo **101,46 µs** de un presupuesto de 5333 µs; cero recortes. Ver `render-probe.json`.

**No verificado**: aspecto visual de ningún estilo, legibilidad real de las waveforms, comportamiento de la vista mini en pantalla, rendimiento del WebView con la consola a 20 Hz, decodificación+análisis con archivos largos reales. CI de tres OS configurada pero sin ejecución verificada.

## Cambios con efecto para el usuario

- Bases de datos existentes migran a v3 al abrir; una versión anterior de la app ya **no** podrá abrir esa base (rechaza versiones futuras sin modificarla). Hacer copia si se necesita volver atrás.
- Pistas importadas antes de v3 muestran waveform plana hasta su primera carga en un deck, que la calcula y guarda.
- La app se llama `curado.app`. Salir por completo de la instancia anterior antes de abrir la nueva; la caché antigua `Curado Visuals.app` puede borrarse a mano.

## Límites importantes

- PCM precargado limitado a 96 MiB por archivo (~4:22 a 48 kHz estéreo); el prefetch de la cola puede sumar un clip residente más.
- Interpolación lineal y velocidad/tono unidos. Sin beatgrid/sync/key lock; loops sin crossfade; techo de amplitud básico.
- UI a 20 Hz: playhead, waveform detallada y platos avanzan en pasos de 50 ms (S2).
- El feedback visual cubre teclado y clics; acciones MIDI no se reflejan aún en la UI (S7).
- Una sola salida estéreo; sin cue de auriculares, ASIO/JACK ni PipeWire nativo.
- Conexión MIDI manual; sin Clock, encoders relativos, LED ni controlador físico probado.
- Sin media keys/Now Playing/MPRIS/SMTC, video, plugins ni servicios remotos (explorador web propuesto en S5/ADR-012).
- `color-mix()` y unidades de contenedor requieren WebKit de Safari 16+.

## Aprendizajes de implementación

El análisis de waveform cabe en el mismo bucle que limpia y copia muestras: no hace falta una segunda pasada ni retener PCM para calcularla, lo que prepara el terreno para streaming. Enviar peaks dentro de `get_library` era el mayor payload de IPC con bibliotecas grandes; pasar a carga por pista y `ArrayBuffer` reduce el coste a lo que se ve. En la UI, el coste dominante no era React sino redimensionar y repintar canvas en cada snapshot; separar «dibujo estático» de «overlay que se mueve» elimina ese trabajo. Las animaciones de feedback con Web Animations evitan renders y reflows por tecla.
