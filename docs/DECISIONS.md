# Decisiones técnicas

## ADR-001 · Rust + Tauri 2 + React + Bun — aceptada

El audio y los servicios de OS necesitan acceso nativo; la interfaz de biblioteca y decks se desarrolla con React. Bun es gestor de paquetes y lanzador de herramientas, no runtime de audio dentro de la app. Tauri usa el WebView del sistema. La UI no es AppKit/WinUI; menús, tray, audio y helper Metal sí usan APIs nativas.

Reconsiderar JUCE/C++ cuando el objetivo inmediato sea routing profesional, plugins, SDKs de controladores o DSP disponible que compense el coste de FFI. La licencia y madurez de cada componente deben evaluarse antes de distribución comercial. No convertir todo el proyecto a JUCE por necesitar una librería DSP C++.

Referencia: [arquitectura Tauri](https://v2.tauri.app/concept/architecture/), [comandos](https://v2.tauri.app/develop/calling-rust/), [tray](https://v2.tauri.app/learn/system-tray/).

## ADR-002 · CPAL 0.16 y Symphonia 0.5.5 — aceptada para beta

Se eligieron APIs conocidas y acotadas. Existen versiones más recientes; no se asume que este baseline provea todos sus backends o mejoras. El lockfile fija las resoluciones reales. CPAL 0.16 usa CoreAudio en Mac, WASAPI en Windows y ALSA por defecto en Linux. ASIO/JACK son opcionales y no están activados aquí. PipeWire puede intervenir vía compatibilidad ALSA/Pulse según sistema; no se anuncia backend PipeWire nativo de este build.

Symphonia `all` habilita los codecs incluidos, no cualquier codec que exista. Mono/estéreo validado por app; AAC-LC depende de perfil/archivo y no se promete HE-AAC/Opus/DRM. No se anuncia gapless de aplicación por tener una biblioteca que lo soporta.

Actualizar tras pruebas de fixtures, API, latencia y todos los OS. Ver [CPAL 0.16](https://docs.rs/cpal/0.16.0/cpal/) y [Symphonia v0.5.5](https://github.com/pdeljanov/Symphonia/tree/v0.5.5).

## ADR-003 · PCM precargado con límites — temporal

Permite cues/seek/loops de implementación sencilla y elimina I/O del callback. El límite de 96 MiB evita lecturas de tamaño ilimitado. A 48 kHz estéreo float32 son unos 262 segundos; a 96 kHz la mitad. Coste: archivos largos rechazados y límite total de memoria aún grueso. Streaming con buffers y ventanas alrededor de cues es la prioridad siguiente; conservar esta ruta para samples cortos.

## ADR-004 · Estado musical nativo; UI por snapshots — aceptada

Transporte y DSP son autoridad del renderer. SQLite sirve a datos persistentes. React conserva navegación, no el reloj. MIDI y tray no dependen de UI. Teclado inicial usa eventos enfocados e IPC directo para mantener accesibilidad y portabilidad. Si las mediciones de latencia no cumplen, añadir capturas nativas por OS y timestamps alineados al reloj de dispositivo. No describir los atajos actuales como sample accurate.

## ADR-005 · SQLite y archivos referenciados — aceptada

SQLite ofrece transacciones y relaciones para playlists, con migración inicial explícita. La app no copia toda la música, modifica tags ni elimina originales. Mover un archivo rompe su referencia; reparación de rutas y file watching quedan pendientes. Cues por pista y modo se guardan; loops/posiciones/estado de decks todavía son efímeros. El esquema v2 usa IDs AUTOINCREMENT: un ID borrado nunca debe identificar otro archivo y heredar cues incorrectos.

## ADR-006 · Metal separado del audio — aceptada para prototipo macOS

AppKit + MTKView demuestra render nativo GPU y aislamiento de la UI. Se pasa telemetría reducida a 20 Hz. Metal no es una optimización del DSP de baja latencia; la GPU sirve para imagen. No mover efectos de bloques cortos a GPU sin medir sincronización, transferencias y carga compartida en Apple Silicon.

Migrar el helper a un bundle firmado antes de distribución. Considerar renderer wgpu con Metal/D3D/Vulkan para paridad visual, manteniendo la misma interfaz de telemetría. [MetalKit](https://developer.apple.com/documentation/metalkit).

## ADR-007 · Honestidad de estado y puertas de calidad — aceptada

«Compila» no demuestra estabilidad en directo, aislamiento perfecto ni soporte físico de controladores. Cada feature debe tener estado verificable y limitaciones. El usuario puede probar esta base local; la etiqueta production ready requiere cumplir QUALITY.

## ADR-008 · Color semántico y cabecera integrada — aceptada

El usuario eligió `#d66154` como base inicial y solicitó claro/oscuro/variaciones/HSV. La paleta deriva de una sola fuente pura TypeScript; CSS y Canvas consumen tokens/paleta, Metal recibe RGB reducido. Separar el acento exacto de su tono para texto conserva el color elegido y permite contraste legible. Persistencia de ajustes en SQLite con caché de primera pintura; nada del tema entra en el motor.

macOS conserva controles nativos con titlebar Overlay y título oculto, integrados en la cabecera. No usar una barra HTML que replique los botones del sistema. La preferencia expresa del usuario es continuar sin Computer Use ni pruebas visuales; pruebas de config/código y compilación no deben presentarse como revisión visual.
