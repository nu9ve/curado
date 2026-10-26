# Registro de evolución

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
