# curado: guía para continuar

Aplicación de audio de escritorio. El nombre se escribe siempre en minúsculas: **curado**. Antes de modificar código, leer `docs/README.md`, `docs/STATUS.md`, `docs/SESSIONS.md` y las secciones relacionadas de `docs/ARCHITECTURE.md`. El estado comprobado prevalece sobre la visión de producto. Comunicación con el usuario en español; código y nombres técnicos en inglés.

## Mapa

- `crates/audio-engine/`: núcleo Rust independiente de Tauri, decodificación, DSP y CPAL.
- `src-tauri/src/`: biblioteca SQLite, comandos, ciclo de vida, MIDI y cola de reproducción.
- `src-tauri/native/CuradoVisuals.swift`: ventana AppKit + MetalKit, compilada únicamente en macOS.
- `src/`: React, interfaz, puente de comandos, distribución de teclado, telemetría, temas/estilos (`lib/theme.ts`), waveforms (`lib/waveform.ts`) y feedback de teclado (`lib/feedback.ts`).
- `crates/audio-engine/src/analysis.rs`: análisis offline de waveform de 3 bandas (decodificación, nunca callback).
- `docs/`: decisiones, contratos, prioridades, pruebas y registro de cambios.

## Preferencias del usuario

- El color inicial/principal es `#d66154`, con temas claro/oscuro/sistema y personalización HSV. Estilos curado/pixel/gótico/y2k/mínimo; el color sigue siendo del usuario dentro de cada estilo.
- Dirección visual: plano, sin cards ni bordes por defecto; chrome continuo (sidebar+topbar+footer) con grano dither y contenido en una hoja elevada lisa; detalles pixel + gradientes suaves; controles visuales (perillas, faders verticales, platos) con feedback animado también por teclado.
- No usar Computer Use ni pruebas visuales de la app, salvo una nueva instrucción explícita. Verificar mediante compilación, pruebas automatizadas y contratos de código.
- macOS usa cabecera integrada con controles nativos; no volver a la franja de título separada.

## Invariantes

1. Callback de audio: sin locks, disco, red, logging, IPC, asignaciones ni liberación de clips. Buffers preasignados; comandos y trabajo por callback acotados. La liberación de PCM va por la cola `retired` al worker.
2. La posición y el estado de transporte pertenecen al motor. La UI muestra snapshots; no usa timers para inventar el avance. MIDI usa el mismo contrato de acciones.
3. Nunca añadir WebAudio como sustituto silencioso del motor. En navegador solo se previsualiza la interfaz.
4. Biblioteca: solo referencias a archivos; retirar pistas o playlists no borra originales. Cargar mediante ID registrado, no mediante rutas arbitrarias desde IPC.
5. Ninguna función se marca «probada» por compilar. Diferenciar implementada, probada automáticamente, comprobada en Mac, pendiente de hardware y pendiente de otros OS.
6. Añadir DSP implica revisar automatización/smoothing, latencia, buffers, bypass, denormales y presupuesto de callback. Cambios de modo implican revisar transporte, cola, samples, foco y teclado.
7. Mantener CSP y permisos mínimos. Sin scripts remotos, comandos shell expuestos ni acceso genérico al filesystem desde JS.
8. No actualizar versiones de dependencias por rutina. Registrar versión y motivo en decisiones; conservar ambos lockfiles.

## Cierre de una sesión

- Actualizar `docs/STATUS.md` con evidencia, límites y próximos pasos concretos.
- Actualizar `docs/CHANGELOG.md`; ajustar arquitectura/decisiones/contratos si cambiaron.
- Verificar `bun run check`, `bun run build`, `cargo fmt --all -- --check`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`.
- Si cambia empaquetado, Swift, bridge o ciclo de vida, construir `.app` y probar contratos relevantes por código. No abrir ni inspeccionar visualmente mientras siga vigente la preferencia del usuario.
- No declarar production ready sin cumplir `docs/QUALITY.md`.
- No commitear, publicar ni modificar archivos musicales del usuario sin instrucción.
