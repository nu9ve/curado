# Calidad y criterios de publicación

La versión 0.1 es una base funcional de prueba local. Para usar la etiqueta production ready deben existir evidencias de las siguientes puertas; ninguna casilla se presume cumplida.

## Puertas

| Puerta       | Criterio                                                                                                           |
| ------------ | ------------------------------------------------------------------------------------------------------------------ |
| Compilación  | Release reproducible desde lockfiles en los tres OS, fmt, lint, types y tests verdes                               |
| Tiempo real  | Callback sin alloc/dealloc/locks; pruebas de carga, jitter y medición física keyboard/MIDI → DAC                   |
| Estabilidad  | Sesiones de 2 h, 6 fuentes + FX, importación concurrente y visuales, sin glitches, crash ni crecimiento de memoria |
| Dispositivos | Hotplug, ausencia de salida, cambio de default, buffers rechazados, sleep/wake y recuperación                      |
| Archivos     | Corpus mono/estéreo 44.1/48/96 kHz MP3/FLAC/WAV/AAC/ALAC/AIFF/Vorbis, corruptos, grandes, unicode y rutas movidas  |
| Persistencia | Reinicio, migraciones, backup/restore, DB corrupta, disco lleno, referencias desaparecidas                         |
| Entrada      | Layout ES/US, foco/text inputs, conflicto/repeat, MIDI real en tres OS y eventos densos                            |
| Distribución | Firma/notarización macOS y helper, firma Windows, paquetes Linux y licencias/notices                               |
| UX           | Tab/foco, Escape en modales, lector de pantalla, contraste, escalado y etiquetas actualizadas tras remap           |

## Comprobaciones automatizadas

```sh
bun run check
bun run build
bun x prettier --check src scripts docs README.md package.json
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Los tests del motor validan transporte, seek/EOF, loops, pánico, crossfader, límites, sample-rate conversion y cola saturada, transición de modos y cues compartidos. La auditoría de allocator verifica cero alloc/dealloc en el render cubierto, incluyendo load/reemplazo/seek/filtro/cue. Hay pruebas de decodificación WAV mono/estéreo y rechazo de basura. SQLite verifica relaciones, deduplicación en playlists, cascada y settings. El teclado verifica combinaciones únicas, Shift-cues, modificadores y filtros de modo. Ver el resultado real más reciente en STATUS.

## Prueba manual local

1. Cargar demo; reproducir A, cambiar a mezcla, reproducir B, mover crossfader y comprobar cada extremo.
2. Guardar cues en posiciones distintas; saltar; marcar IN/OUT; activar/desactivar loop; variar rate, EQ, filtro, echo y master.
3. Disparar Z/X/C/V mientras suenan los decks; repetir teclado desde foco en botones e inputs.
4. Importar fixture 44.1 kHz y archivo 48 kHz; comprobar duración y que no cambie pitch al cambiar de sample rate de archivo.
5. Crear playlist, añadir/quitar pista, favorito, buscar; salir/reabrir y verificar persistencia. «Retirar» nunca debe borrar original.
6. Escuchar selección; dejar terminar una pista; comprobar avance con ventana oculta. Pausa no debe avanzar; pánico debe detener la cola.
7. Abrir Metal, minimizar/cerrar/reabrir; comparar CPU/GPU y continuidad de audio. Cerrar la ventana principal y controlar desde tray.
8. Elegir otro output/buffer. Configuración fallida conserva salida anterior; configuración válida descarga slots y permite recargarlos.
9. Con controlador físico, aprender nota/CC, reiniciar y verificar mapeo; desconectar/reconectar; revisar dropped/error.

## Medición

No usar RSS de un proceso como memoria total de Tauri: WebKit y GPU tienen procesos propios. No usar porcentaje de CPU como prueba de ausencia de glitches. Presupuesto por callback = frames/sample_rate (256/48000 ≈5.33 ms); comparar P50/P95/P99 y máximo bajo carga. Medir latencia total con loopback y timestamps: buffer + OS + hardware + scheduling.

Las pruebas offline no validan DAC, jitter de hardware ni audio percibido. No afirmar que un backend está en modo exclusivo, usa ASIO, tiene prioridad RT adicional o es zero-copy sin comprobar su configuración real.

## Riesgos conocidos

Interpolación lineal puede introducir aliasing al cambiar rate; límites de loop no tienen crossfade; DSP es básico, no ecualización ni limitación de calidad profesional. Las discontinuidades manuales se atenúan con 128 frames y los parámetros principales con smoothing, pero necesitan pruebas auditivas. Scope de 64 muestras y RMS son una visualización, no un analizador espectral. Un archivo corrupto puede agotar tiempo de decodificación antes de alcanzar el límite de PCM; falta cancelación/timeouts y fuzzeo. Modo y mapeos MIDI comparten política nativa; falta validar la ergonomía con controladores físicos.

Las pruebas de apariencia cubren matemática y contratos de color/configuración, no screenshots. El usuario pidió explícitamente no continuar con revisión visual ni Computer Use. La cabecera Overlay y nuevos temas se compilan y se prueban por configuración, sin inspección visual de esta entrega.
