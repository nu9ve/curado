# Registro de evolución

## 2026-10-26 · BPM/key, cuantizado, sync, FX e instalación local

Análisis offline acotado a 90 s (`music.rs`): BPM/offset con ataques/autocorrelación/refinamiento y key mayor/menor por FFT/chroma/perfiles. Metadata versionada por ID en settings, sin cambio de esquema; se guarda al importar/cargar y se puede corregir/reanalizar. Biblioteca muestra BPM y key americana/Camelot. Decks añaden master, sync, tempo, Q activo por defecto, loops por beats, transposición, key lock, key sync y editor de grid/key.

El renderer conserva el reloj: loops en límites de beat, seguimiento del tempo/fase y cue del follower diferido hasta el siguiente beat bajo SYNC+Q. Cambios de contexto y pánico cancelan cues en espera. Tuning se prepara en un worker con Signalsmith Stretch 0.1.3 MIT, sin FFT/C++ en callback; conserva cantidad de frames y compensa latencia. Se agrupan controles rápidos, cancelan resultados antiguos, valida el lote con tickets/revisiones y confirma al terminar el bloque, incluso al descartar. Se mantiene el transporte y el audio anterior hasta aplicar el nuevo resultado. Hay fallos/reintento explícitos.

Mixer: HP independiente, echo con divisiones de beat, interpolación del delay y smoothing por sample rate; se limpian estados diminutos para evitar denormales en FX. Delay preasignado de ocho segundos, sin reservas en callback. Se mantiene EQ básico y techo; no se presentan como DSP de mastering.

Nuevo instalador `desktop:install`: compilación por defecto, destino por usuario, verificación de identidad/firma/hash, staging y rollback, registro LaunchServices sin abrir/cerrar apps. Se añadió a AGENTS y documentación. Avisos MIT incluidos en bundle. Dependencia nueva exacta y motivo en ADR-014; instalación en ADR-015.

Evidencia y límites en STATUS: 22 tests TS, 38 Rust, probe de callback y de preparación musical. Sin revisión visual/audio físico ni precisión certificada sobre música real. Streaming y presupuesto global siguen siendo prioridades estructurales.

## 2026-10-26 · Ayuda contextual y transporte fiable A/B

Petición: ayuda fija y expandible en el footer, y diagnóstico del deck B silencioso al reactivar cues rápidamente. La captura mostraba B pausado. Dos regresiones reprodujeron el contrato anterior: `Cue` solo buscaba y no iniciaba reproducción tras pausa/EOF. Se cambió a salto + play atómico, sin modificar el transporte del otro deck. Seek/nudge conservan pausa; saltar fuera del loop lo desactiva.

Se añadieron play/pausa/cue conjuntos A+B en una sola entrada de cola y botones en el mixer; no ajustan BPM, no afectan samples ni restauran ganancias/master. Los decks muestran diagnóstico de transporte/routing y los cues muestran su tiempo inicial 0:00. Waveforms detalladas accesibles por teclado.

La cola usa carga+play atómica y repeat-one mediante un solo trigger, evitando estados parciales bajo saturación. Precargas etiquetadas por revisión para descartar resultados/errores de sesiones antiguas y limitar reintentos de archivos fallidos.

`ContextHelp` delega hover y foco; `hints.ts` asocia textos al identificador exacto de cada control, muestra atajos guardados y ofrece fallback de etiqueta/título. El fondo adapta el tono al deck/tema; expandir reserva altura en el footer y recoloca los avisos. Escape cierra el panel sin pánico. Sin nuevas dependencias ni permisos.

Pruebas de regresión: cues tras EOF, señales L/R de ambos decks con retriggers a 44.1/48/96 kHz, samples independientes, PCM compartido con posiciones/ganancias separadas, loops, límites de cola y allocator de callback. TypeScript comprueba asociación de cues/mapeos, diagnóstico y disclosure accesible. Evidencia final en STATUS. No se abrió la app, no se hizo Computer Use ni se comprobó audio físico. Las cinco entregas mayores siguen pendientes según ROADMAP.

## 2026-10-25 · Base 0.1

Directorio inicialmente vacío. Se creó workspace Rust con motor independiente, shell Tauri 2 y UI React/Vite/Bun. Decodificación Symphonia, CPAL, seis slots, dos decks + cuatro samples, DSP básico, controles validados/colas acotadas, reclamación fuera del callback, suavizado de parámetros y transición de transportes manuales.

Se añadieron SQLite con esquema versionado y creación transaccional, biblioteca por referencia, playlists/favoritos, importación validada, atajos remapeables, MIDI Learn, tray/menu bar, recuperación manual de dispositivo y cola de escucha en Rust. Visual integrado a 30 FPS y helper AppKit/MetalKit con shaders y telemetría reducida.

Aprendizajes: disponer de Bun/Rust/Xcode permitió generar un bundle ARM64 local pequeño. El baseline CPAL elegido no habilita PipeWire nativo ni ASIO/JACK; no trasladar al producto todas las capacidades listadas en versiones distintas. Los locks de servicios necesitan un orden explícito: snapshots liberan output antes de consultar MIDI/cola. El callback nunca debe liberar la última referencia de PCM. Streaming y un presupuesto total de memoria son la siguiente mejora estructural.

Estado y evidencia final de la sesión: [STATUS](STATUS.md). Visión de producción: [QUALITY](QUALITY.md).

### Ampliación solicitada: coral, temas e integración nativa

Base exacta `#d66154`, modos claro/oscuro/sistema, variaciones neutra/tonal/contraste, presets, hex y HSV. Se sustituyeron colores de interfaz por tokens semánticos, se añadieron contraste adaptativo, caché inicial y persistencia versionada en SQLite. Canvas y Metal reciben el nuevo acento. Icono regenerable por geometría con la base coral. macOS usa titlebar Overlay y título oculto, conservando controles del sistema.

Se implementaron además cues atómicos compartidos por pista con autosave fuera del callback, tickets de confirmación de cargas, repetición de cola visible y política nativa/persistente de modos para UI, teclado y MIDI. La migración SQLite v1→v2 mantiene referencias y evita reutilizar IDs borrados. Se añadió un caché de módulos Swift dentro de OUT_DIR para builds reproducibles sin depender de cachés globales.

El usuario pidió detener las pruebas visuales; los cambios posteriores se verifican exclusivamente con compilación y tests. Contraste matemático y flags de ventana comprobados no equivalen a revisión visual.

Se corrigió también el empaquetado local: el resource seal del bundle se valida y, sin identidad de publicación, se firma ad hoc de forma reproducible al terminar `desktop:build`. Se respeta cualquier identidad configurada. El crate desktop solo genera `rlib`, evitando archivos estáticos grandes que agotaron el disco durante pruebas. Los tests finales y las medidas offline están en STATUS.

## 2026-10-25 · Rediseño, waveforms de 3 bandas y coordinación

Revisión general solicitada por el usuario: rendimiento, coordinación entre actores, estilo y usabilidad. Nombre visible en minúsculas: **curado**.

**Motor y servicios.** Nuevo `analysis.rs`: waveform `[peak, low, mid, high]` a 50 bins/s calculada durante la decodificación con los cortes del EQ. Esquema SQLite v3 (`tracks.waveform BLOB`) con backfill al cargar pistas antiguas; comando `track_waveform` devuelve bytes crudos. `get_library` deja de enviar peaks. La cola decodifica la siguiente pista 15 s antes del final y repeat-one hace seek en vez de redecodificar. MIDI: EQ con unidad en CC 64 (antes el centro equivalía a −2,4 dB y no se alcanzaba +6 dB). Se eliminó una doble desactivación de cola en `audio_action`.

**Interfaz.** Shell con chrome continuo (sidebar + topbar + footer) y contenido en una hoja elevada; sin cards. Vista Mezcla como consola distribuida como el teclado: deck A izquierda, mixer central (perillas HI/MID/LOW/FILTER/ECHO, faders verticales, medidores LED en dB, crossfader), deck B derecha; waveforms detalladas apiladas; platos que giran con la posición del motor; pitch vertical; samples y mapa de teclado clicable. Feedback por tecla (control iluminado, keycap y HUD). Vista general de waveform pintada una vez con overlays por transform; vista detallada por tiles. Vista mini siempre encima. Escape cierra Configuración.

**Apariencia.** Tema v2: estilos curado, pixel, gótico, y2k y mínimo; tipografía, textura y estilo de waveform configurables; color del usuario dentro de cada estilo. Texturas dither Bayer generadas por tema. Fuentes locales Jersey 15, Pixelify Sans, Geist Mono, UnifrakturMaguntia y Michroma (Fontsource 5.3.0). Logo pixel (anillo «c» + eje) compartido entre UI, `docs/assets/logo.svg` e icono regenerado.

**Documentación.** Nuevo `SESSIONS.md` con diez temas para sesiones dedicadas; ADR-009 a ADR-012; README público.

Aprendizajes: separar dibujo estático de overlays móviles es la mayor mejora de coste en la UI; el análisis cabe en el pase de decodificación; contraste por estilo se prueba sobre ambos materiales (chrome y sheet). Nada de lo visual se inspeccionó en pantalla por preferencia del usuario.

### Ajustes tras primera revisión visual

El usuario autorizó revisar visualmente. Se revisó la interfaz en la vista previa web (`bun run dev`, sin audio) a 1100 y 1440 px. Correcciones: arrastrar perillas/faders seleccionaba texto (WebKit necesita `-webkit-user-select`; además `preventDefault` + foco manual en `pointerdown`); grano dither del chrome a 1 px por texel y opacidad 0,10 (antes 2 px y 0,28, competía con el texto); el mixer pasa a superficie lisa sin grano; perillas con tapa física, anillo LED con glow y el valor sustituye a la etiqueta solo mientras se toca (menos texto en pantalla); faders con grip; decks con plato y pitch arriba y pads/transporte a todo el ancho (antes se aplastaban a 1440 px). Pendiente: revisar en la app nativa con pistas cargadas y los estilos pixel/gótico/y2k/mínimo.
