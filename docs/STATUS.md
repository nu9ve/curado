# Estado de curado

Actualizado: **2026-10-26** (America/Hermosillo). Versión **0.1.0** con BPM/key estimados, beatgrid editable, sync, cuantizado y transposición preparada fuera del callback. Base funcional de pruebas locales; no se ha completado la matriz de producción de QUALITY.

## Para retomar

1. Leer este documento, ARCHITECTURE, ROADMAP y [SESSIONS](SESSIONS.md).
2. Preferencia: **sin Computer Use** salvo instrucción explícita. El 2026-10-25 el usuario autorizó una revisión visual: se hizo en la vista previa web (sin audio, sin pistas) del estilo curado en mezcla y reproductor. La app nativa con pistas reales y los otros cuatro estilos siguen sin revisión visual.
3. Nombre siempre en minúsculas: **curado**. Color inicial exacto **#d66154**, estilo **curado**.
4. Siguiente prioridad estructural: S1 streaming con presupuesto de memoria. Siguiente prioridad de experiencia: S2 reloj de posición (playhead/platos a 60 FPS sin violar el invariante 2).

## Sesión navegador integrado · 2026-10-26

La vista **radio / web** ahora contiene un WebView nativo hijo de la ventana principal, con fractos radio seleccionado inicialmente. Sustituye las ventanas externas descritas en la sesión previa. Conserva sidebar/topbar/footer; sigue el rectángulo reservado por React con ResizeObserver y resize/scroll. Configuración y el diálogo de playlists ocultan el navegador para no quedar debajo; el audio continúa mientras esos diálogos están abiertos. Salir de la vista, entrar a mini, detener audio o cambiar de servicio cierra el WebView y detiene su reproducción; se abre una nueva sesión al volver.

La capability se limita por **webviews: [main]**, sin selección por ventana: un permiso aplicado a la ventana main incluiría a sus hijos. Sigue la validación de label por comando, las URLs fijas/hosts exactos, incognito y el bloqueo de popups/descargas. Tray e instancia única usan Window en lugar de WebviewWindow, que deja de representar main mientras hay hijos. No hay permisos genéricos para crear WebViews desde JS ni cambios de CSP.

Tauri 2.12.5 ya resuelto requiere la feature `unstable` para `Window::add_child`; se habilita sin actualizar dependencias/lockfiles. ADR-012 registra el motivo y el límite de estabilidad. Audio web fuera del mixer nativo, sin inyección de scripts. Implementación y pruebas por código; sin abrir la app ni validar aspecto, foco real, cookies/login o reproducción remota. Evidencia: `bun run check` **24 tests TS**, `cargo test --workspace --locked` **42 Rust** (27 motor + 15 app), clippy con -D warnings, formato y diff check correctos. Build frontend y `.app` ARM64 **13,48 MiB** correctos, firma ad hoc verificada; no notarizada. Instalación/registro correctos en `/Users/nu9ve/Applications/curado.app`, ejecutable SHA-256 `6244aa4761012a314267bce70b140fc43bae827a5aeebdecbb8538d14c685182`. No se abrió ni se validó lanzamiento/indexación Spotlight/Raycast.

## Sesión radio/web previa · 2026-10-26

Implementado lanzador **radio / web** con ventanas aisladas para `https://www.fractos.app/es/radio` y SoundCloud Discover. HTTP público: fractos redirige a www por HTTPS y responde 200. El propietario confirma iframes de YouTube/SoundCloud: no hay fuente PCM ni mezcla nativa. Cambiar vista conserva los decks/cola. Cerrar una ventana remota la destruye; la UI puede cerrar ambas para detener el audio web. Master, FX, sync y pánico siguen controlando solo audio local.

URLs fijas, política HTTPS por hosts exactos (widgets incluidos), incognito, popups/descargas denegados. Manifiesto explícito de 26 comandos; ACL solo main y defensa adicional por label, sin permisos remotos ni cambios CSP/core. Pruebas: **24 tests TS y 41 Rust** (27 motor + 14 app); clippy limpio. `bun run build` (vía desktop:build), `bun run format:check`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings` y `git diff --check`: correctos. `.app` ARM64 de **13,44 MiB**, firma ad hoc verificada; no notarizada.

`bun run desktop:install --no-build`: instalada y registrada en `/Users/nu9ve/Applications/curado.app`, SHA-256 del ejecutable `0996a7b8750239d93a3c1114133c9be6130c0b51ace2f13d09f0d26279a7d178`. No se abrió; lanzamiento e indexación Spotlight/Raycast pendientes de comprobar.

**Pendiente de comprobar**: reproducción real de embeds, cookies/login y navegación en WebView de cada OS. No se abrió la app ni se hizo revisión visual/audio. Un HTTP 200 y tests de política no validan compatibilidad de players. Login por popups/hosts externos bloqueado; sesión no persistente. Siguiente integración posible: transporte/volumen vía protocolo con fractos y APIs de widgets; EQ/BPM/key/sync nativos requieren fuentes directas autorizadas y streaming. Ver ADR-012/S5.

## Sesión musical · 2026-10-26

Implementados análisis de BPM/offset y tonalidad mayor/menor, notación americana/Camelot en biblioteca/decks y editor de BPM/offset/key con reanálisis. Nuevas importaciones analizan hasta 90 s; pistas anteriores se analizan al cargar. Metadata versionada en settings por ID; las correcciones manuales se conservan, sin migración ni modificación de archivos musicales.

Q está activo por defecto. Loops de 1/2/4/8/16 beats se alinean al grid y evitan colas parciales; sin grid se conserva el loop en segundos. Sync nativo sigue BPM/fase del master. Un cue del follower con SYNC+Q espera al siguiente beat, se ejecuta por muestra y se cancela con pánico/pausa conjunta/cambios de contexto. Transposición ±12 semitonos, key lock y key sync puntual (tónica o relativa) usan PCM preparado en worker con Signalsmith Stretch 0.1.3 MIT; el audio anterior continúa hasta estar listo. Se validan tickets/revisiones y se confirma la aplicación o descarte del lote. Nuevos FX: HP y echo en beats con smoothing y delay preasignado de 8 s por slot.

`bun run desktop:install` compila e instala en `~/Applications/curado.app`; `--no-build` reutiliza el bundle verificado y `--check` comprueba sin instalar. El script valida identidad/firma/hash, reemplaza con rollback y registra LaunchServices. AGENTS incorpora su ejecución al cierre. No abre la app ni fuerza el cierre de la instalada. La indexación de Spotlight/Raycast y el lanzamiento desde esos programas no se consideran probados por registrar el bundle.

### Evidencia de esta sesión

- `bun run check`: **22 tests**, incluidos los 24 códigos Camelot/transposición y rollback real del instalador en un directorio temporal.
- `cargo test --workspace --locked`: **38 tests** (27 motor + 11 app): clicks de 90/120/124/150 BPM dentro de ±0,1 BPM y offset a ±20 ms; triadas mayores/menores en las 12 raíces; sync tempo/fase con loop; cue diferido y cancelación por pánico; transposición de 220→440 Hz, key lock a 1,25×, stereo y longitud preservados; tuning obsoleto/reintento/recarga del mismo ID; HP, delay rítmico, metadata SQLite y allocator ampliado a nuevos comandos.
- Build frontend y `.app` ARM64 correctos, firma ad hoc verificada; avisos MIT del wrapper, Stretch y Linear incluidos en recursos. CPP/libclang se usan en build; CPP/FFT no entran al callback. Sin nuevos permisos Tauri/CSP.
- `bun run desktop:install --no-build`: **instalada y registrada** en `/Users/nu9ve/Applications/curado.app`. SHA-256 del ejecutable: `c1bd4b0d85e532f13b375f29a32f433c642509473a600bc899490de975f7cd68`. Bundle de 13,31 MiB; no se abrió. Registro/copia verificados, lanzamiento e indexación desde Raycast/Spotlight pendientes del usuario.
- Probe offline con seis fuentes/FX: P50 **36,67 µs**, P95 **39,38 µs**, P99 **51,29 µs**, máximo **147,25 µs** de 5333,33 µs; cero recortes. No borra el outlier histórico de la sesión anterior ni valida salida física.
- Probe musical sintético (30 s/48 kHz): análisis **22,76 ms**, preparación (rate 1,25× y +2 semitonos) **457,12 ms**, BPM estimado **119,943** para 120, 1.440.000 frames conservados y 23.040.000 bytes de PCM entre fuente/resultado. No es RSS total ni latencia DAC. Ver `music:probe` y `musical-probe-2026-10-26.json`.

**Límites vigentes:** no se ha escuchado/inspeccionado esta entrega ni validado con corpus real/controlador. El análisis es heurístico, no detecta tempo variable/downbeats/modulaciones; los indicadores no son probabilidades calibradas. Loops sin crossfade. Key lock/transposición preparan clips completos: pueden tardar segundos y añadir hasta 192 MiB de fuentes + 192 MiB de resultados, además de los clips existentes; falta presupuesto global. El límite de 96 MiB por archivo sigue vigente. No hay timestretch continuo para pistas largas, recuperación automática ni matriz de producción completa.

## Implementado

| Área                | Estado y alcance                                                                                                                        |
| ------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| Stack               | Bun + React/TS/Vite + Tauri 2; crate de audio Rust independiente                                                                        |
| Reproducción        | CPAL, Symphonia, mono/estéreo, 2 decks y 4 one shots, seek, fin de pista                                                                |
| Mezcla              | Equal power, master, EQ básico, LP/HP, echo en beats, tempo, nudge, loops IN/OUT                                                        |
| Waveforms           | Análisis offline 3 bandas a 50 bins/s en la decodificación, BLOB SQLite v3, IPC binario, vista general + detallada (8 s)                |
| Cues                | Tres por track, estado atómico compartido entre decks, autosave SQLite y restauración al cargar                                         |
| Biblioteca          | Archivos/carpetas, metadatos, búsqueda, favoritos, playlists; listado sin peaks (escala con bibliotecas grandes)                        |
| Cola                | Worker Rust, avance nativo, repeat off/one/all; **prefetch** 15 s antes del final; repeat-one sin redecodificar; sin gapless            |
| Modos               | Player/mix persistentes, política nativa compartida por UI/teclado/MIDI                                                                 |
| Entrada             | Atajos físicos remapeables con **feedback visual** (control, keycap y HUD), mapa de teclado clicable, MIDI Learn (EQ unitario en CC 64) |
| UI                  | Chrome continuo + sheet elevada; consola A · mixer · B con platos, perillas, faders verticales y medidores LED en dB                    |
| Vistas              | Completa y **mini siempre encima** (460×132) con restauración de tamaño                                                                 |
| Apariencia          | Estilos curado/pixel/gótico/y2k/mínimo, claro/oscuro/sistema, variaciones, tipografía, textura (dither/suave/plano), waveform           |
| Marca               | Nombre en minúsculas; logo pixel (anillo «c» + eje) en UI, SVG y set de iconos regenerado por geometría                                 |
| OS                  | Tray/menu bar, cerrar para ocultar si hay tray, recuperar dispositivo/buffer manualmente                                                |
| Datos               | SQLite WAL, esquema v3, migraciones v1→v2→v3 probadas, IDs estables, originales conservados                                             |
| Ayuda               | Footer fijo con hover/foco, función, atajos remapeados y detalles expandibles; diagnóstico de transporte/routing por deck               |
| Transporte conjunto | Play/pausa/cue A+B en un único comando; carga+play de cola atómica; precarga con revisión y fallo acotado                               |

## Sesión previa · footer y transporte A/B

El caso reportado (B en silencio al pulsar cue mientras A suena) se reprodujo por código: el cue anterior cambiaba posición sin iniciar un deck pausado/terminado. Las regresiones fallaron antes del cambio. Ahora salto + play son un único comando por slot, sin alternar ni modificar el otro deck. No se constató una carrera por compartir el dispositivo de salida: ambos decks ya se suman en el renderer y conservan posición, reproducción y ganancias independientes.

Se añadieron acciones A+B en el mismo límite de bloque, pruebas a 44.1/48/96 kHz con señales separadas L/R y samples en marcha, y cobertura del límite de 64 comandos. La carga+play de la cola ya no puede aceptarse a medias si solo queda una entrada libre. Las precargas se identifican por revisión de sesión; resultados/errores antiguos no alteran una cola nueva, y un fallo no se reintenta a 20 Hz (hay un último intento al avanzar y se reinicia la política al iniciar otra cola).

El footer resuelve acciones exactas y mapeos actuales; controles genéricos tienen fallback de etiqueta/título. Expande dentro del chrome sin tapar la hoja; Escape desde la ayuda no envía pánico. La vista mini no cambia. La implementación y el markup se verifican automáticamente; **el nuevo footer y sus estados no se han inspeccionado visualmente ni escuchado en hardware**.

Al terminar esa sesión seguían pendientes las cinco entregas mayores. La sesión musical posterior implementa una base de la entrega 5; streaming, importación cancelable, reloj fluido, sesión, DSP avanzado, recuperación y validación de producción siguen pendientes.

### Evidencia de esa sesión · 2026-10-26

- `bun run check`: **20 tests TypeScript**, 11.459 assertions, sin fallos. Incluye ayuda por acción/cue exactos, remapeos, diagnósticos, modo y markup de expansión accesible.
- `cargo test --workspace --locked`: **26 tests Rust** (16 motor + 10 app), sin fallos. Incluye allocator con A/B + sample, cues conjuntos, carga+play, EOF, PCM compartido y precargas obsoletas/fallidas.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`, `bun run format:check` (incluye `cargo fmt --all -- --check`) y `git diff --check`: correctos.
- `bun run build` (también ejecutado por `desktop:build`): JS **329,87 KB / 104,76 KB gzip**, CSS **51,08 KB**. Sin nuevas dependencias ni permisos.
- `bun run desktop:build`: `.app` ARM64 de **13,10 MiB**, firma ad hoc y verificación local correctas; sin abrir la app. No notarizada.
- `bun run probe`: dos corridas conservadas en [render-probe-2026-10-26.json](render-probe-2026-10-26.json). Tras terminar el build: P50 **16,75 µs**, P95 **18,21 µs**, P99 **63,37 µs**, máximo **52.988,25 µs**, cero recortes. **El máximo excede el presupuesto de 5333,33 µs**. No se conoce la causa del outlier y el host no estaba aislado; la medición es tiempo de pared, no prueba de dropouts físicos ni garantía de continuidad. Queda pendiente medir scheduling/carga y salida real según QUALITY.

## Evidencia automatizada · baseline 2026-10-25

- `bun run check`: **15 tests TypeScript** en 4 archivos, 11.404 assertions: paleta en 5 estilos × 10 colores × modos × variantes con contraste sobre chrome y sheet, migración de tema v1→v2, tokens/variables CSS, permisos de ventana exactos, política de modo del teclado, formato binario de waveform, matriz Bayer y geometría del logo.
- `cargo test --workspace`: **19 tests Rust** (10 motor + 9 app): allocator sin alloc/dealloc en render, análisis de bandas (60 Hz → graves, 9 kHz → agudos, 50 bins/s a 44.1 y 48 kHz), migración v1→v3 con backfill sin sobrescritura, prefetch de cola, repeat-one por seek, tema v1→v2 en Rust.
- `cargo clippy --workspace --all-targets -- -D warnings`: limpio. `cargo fmt --all -- --check` y `bun run format:check`: limpios.
- `bun run build`: tipos y frontend de producción (JS 318 KB, 101 KB gzip; CSS 48 KB; fuentes woff2 ~120 KB).
- `bun run desktop:build`: `target/release/bundle/macos/curado.app` ARM64 de **13,09 MiB**, firma ad hoc y `codesign --verify --deep --strict` correctos. `CFBundleName` = curado, identificador sin cambios.
- Probe offline (render sin cambios): P50 **16,75 µs**, P95 **17,38 µs**, P99 **18,12 µs**, máximo **101,46 µs** de un presupuesto de 5333 µs; cero recortes. Ver `render-probe.json`.

**No verificado**: aspecto de los estilos pixel/gótico/y2k/mínimo y del modo claro, legibilidad real de las waveforms, comportamiento de la vista mini en pantalla, rendimiento del WebView con la consola a 20 Hz, decodificación+análisis con archivos largos reales. CI de tres OS configurada pero sin ejecución verificada.

## Cambios con efecto para el usuario

- Hot cues (incluido el botón CUE) ahora saltan **y reproducen**, también desde pausa/EOF; Shift + cue guarda sin iniciar. Seek conserva pausa. Cualquier salto fuera de un loop lo desactiva. Los pads muestran 0:00 como posición inicial en lugar de un guion.
- Bases de datos existentes migran a v3 al abrir; una versión anterior de la app ya **no** podrá abrir esa base (rechaza versiones futuras sin modificarla). Hacer copia si se necesita volver atrás.
- Pistas importadas antes de v3 muestran waveform plana hasta su primera carga en un deck, que la calcula y guarda.
- La app se llama `curado.app`. Salir por completo de la instancia anterior antes de abrir la nueva; la caché antigua `Curado Visuals.app` puede borrarse a mano.

## Límites importantes

- PCM precargado limitado a 96 MiB por archivo (~4:22 a 48 kHz estéreo); el prefetch de la cola puede sumar un clip residente más.
- Interpolación lineal en reproducción; EQ y techo básicos; loops sin crossfade. Transposición/key lock se preparan por clip y no son DSP continuo de baja latencia.
- UI a 20 Hz: playhead, waveform detallada y platos avanzan en pasos de 50 ms (S2).
- El feedback visual cubre teclado y clics; acciones MIDI no se reflejan aún en la UI (S7).
- Una sola salida estéreo; sin cue de auriculares, ASIO/JACK ni PipeWire nativo.
- Conexión MIDI manual; sin Clock, encoders relativos, LED ni controlador físico probado.
- Sin media keys/Now Playing/MPRIS/SMTC, video ni plugins. Explorador remoto aislado implementado (S5/ADR-012); reproducción web pendiente de comprobar y sin integración con el motor.
- `color-mix()` y unidades de contenedor requieren WebKit de Safari 16+.

## Aprendizajes de implementación

El análisis de waveform cabe en el mismo bucle que limpia y copia muestras: no hace falta una segunda pasada ni retener PCM para calcularla, lo que prepara el terreno para streaming. Enviar peaks dentro de `get_library` era el mayor payload de IPC con bibliotecas grandes; pasar a carga por pista y `ArrayBuffer` reduce el coste a lo que se ve. En la UI, el coste dominante no era React sino redimensionar y repintar canvas en cada snapshot; separar «dibujo estático» de «overlay que se mueve» elimina ese trabajo. Las animaciones de feedback con Web Animations evitan renders y reflows por tecla.
