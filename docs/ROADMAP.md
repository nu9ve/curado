# Prioridades y dependencias

Los temas grandes tienen su ficha en [SESSIONS](SESSIONS.md) (S1…S10).

## Secuencia de entregas acordada · 2026-10-26

La corrección de hot cues, transporte A+B y base de ayuda contextual precede estas entregas. Esa sesión no completó streaming, reloj fluido ni sync musical; la sesión musical posterior añade la base de sync/beatgrid/key lock descrita en STATUS. Ninguna entrega sustituye las puertas de QUALITY.

| Entrega                         | Dependencias                              | Cierre verificable pendiente                                                                                                   |
| ------------------------------- | ----------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| 1 · Pistas largas e importación | S1                                        | Archivo de 60 min con seek/loops y memoria global acotada; progreso, cancelación e importación que cede a cargas prioritarias. |
| 2 · Transporte fluido y sesión  | Reloj nativo S2; almacenamiento de sesión | Playhead fluido sin deriva; confirmación de acciones; restauración pausada de decks y loops sin snapshots incoherentes.        |
| 3 · Calidad y recuperación      | S3; ciclo de vida de dispositivos         | Medidas de aliasing/discontinuidad, limitador, coste RT; hotplug/sleep-wake y cierre determinista.                             |
| 4 · Reproductor cotidiano       | Biblioteca y cola robustas                | Cola editable, shuffle, playlists reordenables, media keys, álbumes/artwork y reparación de rutas.                             |
| 5 · Mezcla avanzada             | Reloj/streaming/DSP; S4 y routing         | BPM/beatgrid editable, loops por beats y sync; después key lock y preescucha hardware.                                         |

CI ejecutada en los tres OS, corpus de codecs/errores, estrés de dos horas, accesibilidad y distribución firmada deben acompañar el avance. S1 continúa siendo la siguiente prioridad estructural; no aumentar el límite de PCM como sustituto del presupuesto de memoria.

## P0 · Antes de estabilidad de producción

1. **Streaming y memoria por presupuesto global** (S1). Decodificación incremental, rings preasignados por deck, prefetch de cues, bounded seek, cancelación y tiempo límite por tarea. Mantener precarga para samples. Añadir corpus de tracks largos y disco lento. Afecta waveform, cola, loops, cue y caché.
2. **Contratos de sesión nativos** y **reloj de posición** para playhead fluido (S2). Modo nativo y cues por track ya implementados. Completar loops/sesión de decks persistentes, ack transaccional de todos los transportes y restauración pausada. No persistir posiciones desde snapshots incoherentes.
3. **Audio de mayor calidad** (S3). Resampler bandlimited, crossfade en loops, rampas medidas por sample rate, EQ biquad, headroom y limitador real. Tests de discontinuidad, aliasing, denormales y coste de callback antes de añadir FX.
4. **Resiliencia de OS/dispositivos**. Hotplug y sleep/wake, reconexión controlada sin sorpresa de volumen; teardown y join de threads determinista, errores recuperables de DB. Medir UI/input/DAC y callback bajo carga.
5. **Distribución y prueba física** (S10). CI ejecutada tres OS, helper firmado dentro del bundle, firmas/notarización, auditoría de dependencias/licencias, dos horas de estrés. Tray y salida explícita en Linux sin indicador.

## P1 · Reproductor completo

- Cola editable y shuffle. Prefetch de la siguiente pista ya implementado; gapless real requiere P0 streaming y reloj nativo.
- Playlists reordenables, tags reales, artwork, álbumes, reubicación de archivos y escaneo incremental cancelable.
- Temas, estilos y modo ya persisten. Completar estado de navegación y salida validada al arrancar.
- Vistas alternas (S6): mini siempre encima ya existe; faltan compacta, foco/escenario, visual flotante y persistencia de tamaño por vista.
- Tray macOS con icono plantilla y «ahora suena» (S8).
- Media keys / Now Playing macOS, SMTC Windows, MPRIS Linux; autostart opt-in y asociaciones de archivo.
- Accesibilidad: foco atrapado en modal y restauración, Escape, contraste de interfaz auditado (paleta ya tiene pruebas matemáticas), escalado dinámico, lector de pantalla.

## P1 · Mezcla más seria

- Base S4 implementada: BPM/key offline, grid editable, loops por beats, sync nativo y cues diferidos. Pendientes corpus real, métricas de precisión, tempo variable y tratamiento de grids dudosos.
- Transposición/key lock inicial mediante Signalsmith preparado offline. Pendientes calidad auditiva en corpus real, procesamiento incremental compatible con streaming y presupuesto global de memoria; conservar compensación de latencia y contratos de transporte.
- Salida cue/master multicanal y selección de pares; ASIO/JACK/PipeWire según plataforma. Requiere cambios de configuración, routing, UI y tests hardware.
- Jog/nudge continuo (arrastrar el plato) y acciones con timestamp, controles relativos MIDI, hotplug, feedback LED y feedback de MIDI en la UI (S7).
- Captura de teclado nativa solo si las mediciones lo requieren; global shortcuts opt-in con análisis de conflictos/permisos.

## P2 · Expansión

- Renderer GPU compartido wgpu → Metal/D3D/Vulkan, FFT por worker y features musicales suavizadas con timestamps; calidad adaptativa por presupuesto GPU.
- Scene presets, waveform 3D, paisajes, grabación/export de sets con permisos explícitos.
- Host de plugins VST/AU solo con aislamiento de fallos y revisión de licencia.
- Explorador web nativo integrado y aislado implementado para SoundCloud / radio de fractos (S5, ADR-012), audio fuera del motor. Pendiente validar players/cookies/login en WebView real y diseñar control de transporte/volumen con fractos; integración de fuentes al motor requiere streaming autorizado.
- Video, red o servicios musicales integrados al motor son proyectos propios, no capacidades implícitas de Symphonia/Tauri.
- Temas compartibles (export/import de un tema v2 validado) y más estilos.

Cada entrega debe actualizar STATUS y CHANGELOG, añadir evidencia para su contrato y reevaluar las funciones afectadas. No lanzar múltiples cambios de DSP, scheduling y visuales a la vez: se pierde atribución de rendimiento y de fallos.
