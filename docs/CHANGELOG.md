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
