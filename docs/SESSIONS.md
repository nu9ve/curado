# Sesiones dedicadas

Temas que no caben como «un cambio más»: cada uno merece una sesión propia con su objetivo, su evidencia y su cierre. Ordenados por prioridad sugerida. Al empezar uno, copiar su bloque a STATUS como «en curso»; al terminar, moverlo al CHANGELOG con la evidencia.

Formato: **contexto** (por qué ahora), **alcance** (qué entra y qué no), **hecho cuando** (criterio verificable), **riesgos**.

## S1 · Streaming y presupuesto global de memoria (P0)

- **Contexto**: PCM precargado limita a ~4:22 por pista y hasta 6 × 96 MiB residentes, más un clip de prefetch de la cola. Es el límite estructural más visible para un reproductor.
- **Alcance**: decodificación incremental por deck con ring preasignado, seek acotado, prefetch alrededor de cues/loops, cancelación y timeouts de decodificación. Mantener precarga para samples. La waveform ya se calcula en el mismo recorrido y no depende del PCM residente.
- **Hecho cuando**: pista de 60 min reproduce, busca y hace loops sin alloc en el callback (test de allocator), memoria total acotada y medida, corpus de disco lento.
- **Riesgos**: seeks lejanos con codecs sin índice; latencia de cue; interacción con el prefetch de la cola.

## S2 · Reloj de posición y playhead fluido (P0/UX)

- **Contexto**: la telemetría a 20 Hz mueve playhead, platos y waveform detallada en saltos de 50 ms. La regla vigente prohíbe inventar avance con timers.
- **Alcance**: decidir entre (a) telemetría 60 Hz solo de posiciones (struct pequeño, sin JSON grande) o (b) snapshot con `timestamp` del callback + rate para extrapolar dentro de un intervalo acotado y corregido por cada snapshot. Documentar la decisión como ADR; la UI nunca debe adelantarse al motor más de un intervalo.
- **Hecho cuando**: waveform detallada y plato a 60 FPS sin deriva medible respecto al motor (test que compara posición extrapolada vs snapshot), coste de IPC medido.
- **Riesgos**: romper el invariante 2 si se extrapola sin acotar; carga del WebView.

## S3 · Calidad de audio: resampler, EQ y limitador (P0)

- **Contexto**: interpolación lineal, EQ de un polo y techo de ±0.98.
- **Alcance**: resampler bandlimited (sinc/polyphase), EQ biquad tipo DJ (kill a −∞), filtro bipolar HP/LP, crossfade en límites de loop, limitador real. Revisar smoothing, denormales, latencia, bypass y presupuesto por callback (invariante 6).
- **Hecho cuando**: tests de aliasing y discontinuidad, probe offline con nuevos P99, sin alloc en callback.

## S4 · Beatgrid, BPM, tonalidad y sync (P1 mezcla)

**Base implementada el 2026-10-26**: análisis offline, grid editable, loops cuantizados, sync nativo, tonalidad americana/Camelot y transposición/key lock preparados en worker. No se cierra la sesión de calidad: faltan corpus real, validación auditiva y loops sin discontinuidades.

- **Contexto**: la waveform de 3 bandas ya existe; falta estructura rítmica.
- **Alcance**: análisis offline de BPM/onsets en el worker de decodificación (junto a `analysis.rs`), beatgrid persistida en SQLite, marcadores de beat en la waveform, loops cuantizados y sync de tempo. La preparación inicial de key lock ya existe; llevarla a streaming incremental y validar su calidad sigue pendiente.
- **Hecho cuando**: corpus con BPM conocido dentro de ±0.1 BPM; loops de 1/2/4/8 beats sin discontinuidad.

## S5 · Explorador web (SoundCloud, radio de fractos) (P2)

**Base implementada el 2026-10-26**: navegador nativo integrado en main (sustituye las ventanas externas), cierre al salir/cambiar servicio, ACL por webview y pruebas de política/bounds. URL de fractos confirmada y HTTP 200 por HTTPS. Pendiente comprobar los players en el WebView real; no hay mezcla nativa.

- **Contexto**: explorar servicios externos desde la app sin abrir el navegador.
- **Alcance**: WebView hijo aislado con lista blanca de dominios, **sin** capacidades IPC (la capability selecciona solo el webview `main`, no la ventana completa), sin acceso a archivos ni comandos. El audio de esos servicios sale por WebKit, **no** por el motor ni el mixer; no se mezcla ni se graba. Ver ADR-012.
- **Hecho cuando**: test de contrato que verifica que la ventana remota no tiene permisos, CSP de la ventana principal intacta, navegación bloqueada fuera de la lista blanca.
- **Siguiente alcance**: validar cookies/reproducción de embeds y compatibilidad de login bajo incognito sin popups; ahora suena y control de volumen/transporte requieren un protocolo con fractos (el usuario lo administra y usa iframes). No hay acceso al PCM a través de sus widgets.
- **Riesgos**: términos de uso de cada servicio; cookies de terceros; confundir al usuario sobre qué suena por el motor.

## S6 · Vistas alternas (P1 UX)

- **Contexto**: ya existe la vista mini siempre encima (460×132). Faltan variantes.
- **Alcance**: vista compacta (biblioteca + reproductor sin visual), modo foco/escenario (solo decks y waveform a pantalla completa), ventana flotante de visual, persistencia del tamaño/posición por vista, atajo para alternar.
- **Hecho cuando**: cada vista conserva transporte, foco y teclado (invariante de cambios de modo); restauración de tamaño probada por contrato.

## S7 · Feedback de MIDI y estado en la UI (P1)

- **Contexto**: las teclas ya iluminan controles y muestran HUD. Las acciones MIDI no, porque ocurren en Rust.
- **Alcance**: evento ligero `input:action` emitido por el worker MIDI (acotado, coalescido) que reutiliza `feedback.ts`; feedback LED hacia el controlador; encoders relativos.
- **Hecho cuando**: con controlador real, cada mensaje mapeado se refleja en la UI sin afectar la latencia MIDI→motor.

## S8 · Tray de macOS con icono plantilla (P1 pulido)

- **Contexto**: la barra de menús usa el icono a color de la app.
- **Alcance**: icono monocromo `icon_as_template`, estado de reproducción en el menú, «ahora suena».
- **Hecho cuando**: icono correcto en claro/oscuro (requiere validación visual explícita del usuario).

## S9 · Accesibilidad y auditoría visual (P1)

- **Contexto**: contraste matemático probado en 5 estilos × 9 colores × modos × variantes; sin revisión visual por preferencia del usuario.
- **Alcance**: foco atrapado en modales, lector de pantalla para perillas/faders, escalado de texto, revisión visual de los 5 estilos cuando el usuario lo autorice.

## S10 · Distribución y CI real (P0)

- **Contexto**: CI preparada, sin ejecución verificada; build local ad hoc.
- **Alcance**: correr CI en los tres OS, helper Metal firmado dentro del bundle, notarización, licencias/notices (incluidas las fuentes OFL), licencia del repositorio.
