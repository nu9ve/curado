# Prioridades y dependencias

## P0 · Antes de estabilidad de producción

1. **Streaming y memoria por presupuesto global**. Decodificación incremental, rings preasignados por deck, prefetch de cues, bounded seek, cancelación y tiempo límite por tarea. Mantener precarga para samples. Añadir corpus de tracks largos y disco lento. Afecta waveform, cola, loops, cue y caché.
2. **Contratos de sesión nativos**. Modo nativo y cues por track ya implementados. Completar loops/sesión de decks persistentes, ack transaccional de todos los transportes y restauración pausada. No persistir posiciones desde snapshots incoherentes.
3. **Audio de mayor calidad**. Resampler bandlimited, crossfade en loops, rampas medidas por sample rate, EQ biquad, headroom y limitador real. Tests de discontinuidad, aliasing, denormales y coste de callback antes de añadir FX.
4. **Resiliencia de OS/dispositivos**. Hotplug y sleep/wake, reconexión controlada sin sorpresa de volumen; teardown y join de threads determinista, errores recuperables de DB. Medir UI/input/DAC y callback bajo carga.
5. **Distribución y prueba física**. CI ejecutada tres OS, helper firmado dentro del bundle, firmas/notarización, auditoría de dependencias/licencias, dos horas de estrés. Tray y salida explícita en Linux sin indicador.

## P1 · Reproductor completo

- Cola editable, shuffle, prefetch y gapless (requiere P0 streaming y reloj nativo).
- Playlists reordenables, tags reales, artwork, álbumes, reubicación de archivos y escaneo incremental cancelable.
- Temas y modo ya persisten. Completar estado de navegación y salida validada al arrancar.
- Media keys / Now Playing macOS, SMTC Windows, MPRIS Linux; autostart opt-in y asociaciones de archivo.
- Accesibilidad: foco atrapado en modal y restauración, Escape, contraste de interfaz auditado (paleta ya tiene pruebas matemáticas), escalado dinámico, lector de pantalla.

## P1 · Mezcla más seria

- BPM/beatgrid offline y beat clock común. No basar sync en timers JS.
- Time stretch/key lock con revisión de calidad, licencia y latencia. Su latencia debe incorporarse a todo el routing.
- Salida cue/master multicanal y selección de pares; ASIO/JACK/PipeWire según plataforma. Requiere cambios de configuración, routing, UI y tests hardware.
- Jog/nudge continuo y acciones con timestamp, controles relativos MIDI, hotplug y feedback LED.
- Captura de teclado nativa solo si las mediciones lo requieren; global shortcuts opt-in con análisis de conflictos/permisos.

## P2 · Expansión

- Renderer GPU compartido wgpu → Metal/D3D/Vulkan, FFT por worker y features musicales suavizadas con timestamps; calidad adaptativa por presupuesto GPU.
- Scene presets, waveform 3D, paisajes, grabación/export de sets con permisos explícitos.
- Host de plugins VST/AU solo con aislamiento de fallos y revisión de licencia.
- Video, red o servicios musicales son proyectos propios, no capacidades implícitas de Symphonia/Tauri.

Cada entrega debe actualizar STATUS y CHANGELOG, añadir evidencia para su contrato y reevaluar las funciones afectadas. No lanzar múltiples cambios de DSP, scheduling y visuales a la vez: se pierde atribución de rendimiento y de fallos.
