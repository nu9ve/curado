# Decisiones técnicas

## ADR-014 · Análisis musical editable, sync nativo y pitch preparado — aceptada (2026-10-26)

El usuario pidió BPM, tonalidad americana/Camelot, transposición, key lock/key sync, cuantizado activo por defecto y loops sincronizados. Se añade análisis offline acotado a 90 s, con autocorrelación/refinamiento de ataques y chroma/perfiles [Krumhansl-Kessler documentados por Humdrum](https://extras.humdrum.org/man/keycor/). No se presume precisión de producto por pasar señales sintéticas. Offset, BPM y key son editables y se guardan por ID en settings, sin nueva migración ni tocar tags originales. El mapeo Camelot sigue la [referencia del creador](https://mixedinkey.com/camelot-wheel/0/); americana/Camelot representan la misma tonalidad.

Se fija la nueva dependencia **signalsmith-stretch =0.1.3**, wrapper Rust y biblioteca C++ bajo MIT. [API y licencia oficiales](https://github.com/Signalsmith-Audio/signalsmith-stretch); avisos incluidos en `licenses/signalsmith-stretch` y en recursos del bundle. Cargo.lock incorpora sus transitivas; no se actualizaron dependencias existentes por rutina y se conserva bun.lock. Compilar requiere C++ y libclang/bindgen además de Rust.

Se eligió preparar PCM fuera del callback, manteniendo longitud/sample rate y compensando input/output latency, para no introducir FFT, C++ ni reservas en tiempo real mientras el motor siga usando clips completos. La conversión de velocidad permanece nativa. Costes aceptados para esta fase: demora al preparar, caché/resultados adicionales y calidad pendiente de corpus/escucha. No es procesamiento continuo de pitch ni resuelve streaming. Al pasar a S1 deberá migrarse a preparación incremental con presupuesto y medición de latencia.

El motor conserva Q, tempo, fases y cues diferidos; React no sincroniza audio con timers. Sync no inicia reproducción ni transforma el modo mayor/menor. Key sync elige la tónica o relativa compatible más cercana, puntualmente. Un lote de tuning se valida con generación/ticket/revisión y se confirma incluso si se descarta para permitir nuevos intentos.

## ADR-015 · Instalación local por script y LaunchServices — aceptada (2026-10-26)

El usuario solicita que cada cierre deje una app local actualizada para lanzar desde Spotlight/Raycast. `desktop:install` compila por defecto y copia a `~/Applications/curado.app` sin sudo. Verifica identificador, firma y hash del ejecutable, prepara la copia antes del reemplazo, usa renames con rollback y registra el destino en LaunchServices. No modifica preferencias, biblioteca ni originales, no abre la app y no fuerza cierres. Si la instalada está abierta, se detiene con un error para evitar reemplazar recursos mientras suena. La indexación del launcher y el lanzamiento manual siguen siendo comprobaciones distintas de copiar/registrar el bundle.

## ADR-013 · Hot cues que reproducen y transporte conjunto — aceptada (2026-10-26)

El usuario observó que B seguía en silencio al reactivar su cue mientras A sonaba. En código, `Cue` cambiaba posición pero conservaba `playing=false` tras pausa/EOF; dos regresiones fallaron antes de corregirlo. Se define el hot cue como salto + reproducción en un único comando. Guardar cue y buscar posición mantienen sus contratos separados. Saltar fuera de un loop lo desactiva.

Los decks ya tienen estado independiente y se suman en un renderer común. Se conserva ese diseño: separar dispositivos/streams no resolvería la semántica de cue y complicaría sus relojes. Para control simultáneo explícito, `playBoth`/`pauseBoth`/`cueBoth` actúan sobre A/B dentro de una sola entrada de cola, incluso en el límite de 64 comandos por callback. No cambian routing ni volumen. Sin garantía de beat sync; la reproducción física y latencia siguen pendientes de hardware.

La cola también usa una entrada para carga+play y una para repetir la pista, evitando aceptación parcial bajo saturación. Una revisión de sesión invalida precargas antiguas (éxito o error). Los fallos de prefetch se acotan a un intento anticipado y otro al avanzar, en lugar de volver a decodificar en cada tick.

La ayuda contextual usa identificadores de acción y mapeos guardados, con fallback de etiqueta/título para controles genéricos. Los textos son revisables en código; la ayuda no infiere acciones a partir de cercanía visual ni envía controles al motor.

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

## ADR-009 · Fuentes locales vía Fontsource — aceptada

El usuario pidió Jersey 15 y una fuente pixel para textos especiales. La CSP prohíbe recursos remotos, así que se empaquetan archivos locales con `@fontsource/*` **5.3.0** (versión exacta, sin rango): Jersey 15 (display), Pixelify Sans 400/600 (etiquetas pixel; sustituye a «pixel basel», que no está disponible en Google Fonts/Fontsource), Geist Mono 400/600 (números, keycaps), UnifrakturMaguntia (display del estilo gótico) y Michroma (display y2k). Solo subconjunto latino (cubre español): ~120 KB woff2 en total. Todas con licencia SIL OFL 1.1; incluir sus avisos al distribuir. La UI general sigue con la fuente del sistema para legibilidad y aspecto nativo. Reconsiderar si se necesita otro alfabeto o una fuente variable.

## ADR-010 · Waveform de 3 bandas offline y binaria — aceptada

Las waveforms se calculan durante la decodificación existente (sin segunda lectura del archivo) a 50 bins/s con `[peak, low, mid, high]` en `u8`, usando los mismos cortes que el EQ. Se guardan como BLOB (esquema v3) y viajan como `ArrayBuffer`. Alternativas descartadas: FFT por bin (más coste, más memoria, sin ganancia visible a esta escala), JSON de floats (10× más bytes en IPC), enviar peaks en `get_library` (no escala). Colores tipo club: graves azul, medios ámbar, agudos blanco; modo RGB y mono opcionales. Reconsiderar al introducir streaming (S1) y beatgrid (S4), que reutilizarán este pase de análisis.

## ADR-011 · Sistema de estilos sobre tokens — aceptada

Los estilos (curado, pixel, gótico, y2k, mínimo) son semillas de `palette()` más reglas CSS por `data-style`; el color base sigue siendo del usuario dentro de cada estilo. Tipografía y textura son independientes y por defecto heredan del estilo. Preferencias en SQLite como tema v2; v1 se migra con valores por defecto en TS y Rust. El dither se genera con matriz Bayer en canvas una vez por cambio de tema; no hay imágenes externas. Se descartó un motor de skins con CSS arbitrario del usuario (riesgo de CSP/inyección y de romper contraste).

## ADR-012 · Explorador web aislado — aceptada, base implementada

Se implementa un lanzador local con ventanas separadas para `https://www.fractos.app/es/radio` y SoundCloud Discover. El propietario confirmó que fractos usa iframes de YouTube/SoundCloud; no es una fuente PCM. El audio sale por el WebView y queda fuera del motor, mixer, sync, grabaciones y pánico nativo. La UI lo indica y permite destruir ambas ventanas para detenerlo; cerrar una ventana remota no la oculta en el tray.

URLs de entrada fijas, navegación HTTPS por hosts exactos de servicio/widgets, sin popups ni descargas, perfil incognito y sin persistencia de sesión prometida. No se modifica CSP local ni se conceden permisos remotos. El manifiesto registra explícitamente los comandos de app para generar ACL; solo `main` recibe sus permisos, con una segunda validación del label en el handler. Omitir la capability remota por sí solo no bastaba para restringir comandos propios con la configuración anterior. Los permisos generados se reconstruyen desde `app_commands.rs` y no se versionan. Tests verifican correspondencia registro/handler/capability y política de URLs; falta comprobar reproducción, cookies y login en el WebView real.

La mezcla por volumen/transporte vía APIs oficiales de iframe requiere un protocolo con fractos y una sesión propia. No proporciona PCM para EQ/sync del motor; esa integración necesitaría fuentes directas autorizadas y streaming nativo. No se inyectan scripts ni se intenta extraer audio de los embeds. Referencias: [capabilities de Tauri](https://v2.tauri.app/security/capabilities/), [SoundCloud Widget API](https://developers.soundcloud.com/docs/api/html5-widget). Ver SESSIONS S5.
