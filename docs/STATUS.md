# Estado de Curado

Actualizado: **2026-10-25** (America/Hermosillo). Versión **0.1.0**, base funcional de pruebas locales. No se ha completado la matriz de producción de QUALITY.

## Para retomar

1. Leer este documento, ARCHITECTURE y ROADMAP.
2. La preferencia vigente es **sin Computer Use ni pruebas visuales**. Usar código, compilación y pruebas automatizadas.
3. Color inicial exacto **#d66154**. Tema en Configuración → Apariencia. macOS tiene cabecera Overlay integrada.
4. Siguiente prioridad estructural: streaming con presupuesto global de memoria y soporte de pistas largas; después resampler bandlimited y routing de preescucha.

## Implementado

| Área         | Estado y alcance                                                                                                     |
| ------------ | -------------------------------------------------------------------------------------------------------------------- |
| Stack        | Bun + React/TS/Vite + Tauri 2; crate de audio Rust independiente                                                     |
| Reproducción | CPAL, Symphonia, mono/estéreo, 2 decks y 4 one shots, seek, fin de pista                                             |
| Mezcla       | Equal power, master, EQ básico, LP, echo, varispeed, nudge, loops IN/OUT                                             |
| Cues         | Tres por track, estado atómico compartido entre decks, autosave SQLite y restauración al cargar                      |
| Biblioteca   | Archivos/carpetas, metadatos, búsqueda, favoritos, playlists crear/renombrar/añadir/quitar/eliminar                  |
| Cola         | Worker Rust, avance nativo, repetición off/one/all visible; sin gapless                                              |
| Modos        | Player/mix persistentes, política nativa compartida por UI/teclado/MIDI y transición con un comando de audio         |
| Entrada      | Atajos físicos visibles/remapeables, Note On y CC MIDI Learn persistente                                             |
| OS           | Tray/menu bar, cerrar para ocultar si hay tray, recuperar dispositivo/buffer manualmente                             |
| Apariencia   | #d66154 inicial, claro/oscuro/sistema, neutro/tonal/contraste, presets, hex, selector y HSV; persistencia versionada |
| Ventana Mac  | Título oculto, titlebar Overlay, controles nativos y zonas de arrastre integradas                                    |
| Visuales     | Canvas 30 FPS y helper AppKit/MetalKit; recibe RMS/scope/acento, independiente del audio                             |
| Datos        | SQLite WAL, esquema v2, migración v1→v2, IDs estables sin reutilización, originales conservados                      |
| Continuidad  | AGENTS, documentación por contratos, roadmap y CI de tres OS preparada                                               |

## Evidencia automatizada

- `bun run check`: **9 tests TypeScript**, incluye 162 combinaciones de paleta, más de 1.300 assertions, HSV/hex, contraste, tokens CSS, configuración de ventana y teclado.
- `cargo test --workspace`: **16 tests Rust** (9 motor + 7 app), incluidos allocator sin alloc/dealloc durante render, cues compartidos/persistencia real al reabrir SQLite, transición de modos, repeat y migración conservando playlists/favoritos/IDs.
- `bun run build`: types y frontend de producción generados.
- `cargo clippy --workspace --all-targets -- -D warnings`: sin warnings de código.
- `cargo fmt --all -- --check` y `bun run format:check`: formato comprobado.
- Compilación ARM64 de `.app` y helper Swift con deployment target macOS 12. La cabecera y los temas nuevos no se han inspeccionado visualmente, por instrucción del usuario.
- CI de macOS/Windows/Linux configurada en `.github/workflows/check.yml`, **sin ejecución remota verificada**.

El bundle ARM64 final ocupa aproximadamente 12,8 MiB. Se comprobó su Info.plist y su sello local con `codesign --verify --deep --strict`; se reparó la firma ad hoc del contenedor y se automatizó ese paso en `scripts/sign-local.ts`. Esto no es notarización ni firma de distribución.

Ruta de bundle: `target/release/bundle/macos/Curado.app`. Para recibir una versión reconstruida, salir completamente de la instancia anterior y reabrir ese bundle; cerrar solo la ventana la oculta y conserva el proceso.

## Evidencia local anterior al cambio de temas

Antes de la instrucción de detener Computer Use se abrió el bundle y se observó CoreAudio 48 kHz, avance de transporte y medidores con señal. Se verificaron los seis slots de demo, ambos decks reproduciendo mediante Q/P, sample Z, cue con Shift+2, loop W y crossfader con flecha. Se creó una playlist y se comprobó que seguía tras reiniciar. Estos pasos corresponden al baseline anterior; no validan visualmente los temas/cabecera ni sustituyen pruebas de MIDI físico o latencia DAC.

El helper Metal se lanzó como proceso. Su UI final no se inspeccionó. La captura histórica `curado-mix.png` pertenece al diseño verde anterior y no representa la apariencia vigente.

## Medición de CPU disponible

Probe offline final: seis slots con FX, 60 s simulados, 48 kHz, bloques de 256 frames. P50 **16,79 µs**, P95 **17,71 µs**, P99 **33,00 µs**, máximo **92,67 µs**; presupuesto **5333,33 µs**, cero muestras recortadas. Ejecutar `bun run probe` para repetir; el resultado guardado está en `render-probe.json`. Es CPU offline, no latencia física, jitter del dispositivo ni prueba de ausencia de glitches en vivo.

## Límites importantes

- PCM precargado limitado a 96 MiB por archivo: aproximadamente 4:22 a 48 kHz estéreo. Streaming, cancelación de importación y límite global por bytes pendientes.
- Interpolación lineal y velocidad/tono unidos. Sin beatgrid/sync/key lock; loops sin crossfade; techo de amplitud básico, no limitador profesional.
- Una sola salida con master L/R; sin cue de auriculares independiente, ASIO/JACK ni backend PipeWire nativo habilitados.
- Conexión MIDI manual; sin MIDI Clock, encoders relativos, feedback LED o controlador físico probado.
- Cues, tema y modo persisten; loops, posiciones de decks, cola y navegación todavía son de sesión.
- Sin media keys/Now Playing/MPRIS/SMTC, video, plugins o servicios remotos.
- macOS 12 antiguo, Windows/Linux, sleep/wake, hotplug y sesiones de dos horas aún pendientes de validación. Build local sin notarización de distribución.
- La paleta tiene contraste matemático probado; la accesibilidad completa de todos los flujos necesita auditoría.

## Aprendizajes de implementación

No llamar listo a una carga solo porque se encoló: ahora se confirma mediante ticket del callback. Los cues no se guardan muestreando posición desde UI; un estado atómico por track permite compartirlos y guardar revisiones exactas fuera del callback. No retener locks en un snapshot compuesto: cada servicio se consulta por separado y se respeta el orden documentado.

El build agotó espacio al generar `staticlib` innecesaria. Se redujo el crate desktop a `rlib` y se limpiaron únicamente artefactos de compilación del paquete. Swift utiliza caché local bajo OUT_DIR; no depender de rutas globales. No editar cachés o archivos ajenos al proyecto para resolver builds.
