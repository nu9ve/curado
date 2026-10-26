<p align="center">
  <img src="docs/assets/logo.svg" width="72" height="72" alt="curado">
</p>

<h1 align="center">curado</h1>

<p align="center">
  Reproductor y mezclador de escritorio con audio nativo en tiempo real.<br>
  Rust para el sonido, una interfaz que se toca con el teclado.
</p>

---

**curado** es una app local para escuchar tu biblioteca y mezclarla en vivo. El audio vive en un motor Rust (CPAL + Symphonia) que nunca bloquea, asigna memoria ni toca disco dentro del callback; la interfaz (Tauri 2 + React) solo muestra lo que el motor reporta y le envía acciones validadas. La biblioteca y mezcla locales funcionan sin conexión y tus archivos nunca se mueven ni se modifican. La vista opcional radio/web requiere internet.

> **Estado: 0.1, en desarrollo activo.** Probado localmente en Apple Silicon (macOS). Windows y Linux comparten el código y tienen CI configurada, pero aún no se validan en hardware. Lee [docs/STATUS.md](docs/STATUS.md) antes de usarlo en serio y [docs/QUALITY.md](docs/QUALITY.md) para lo que falta antes de llamarlo estable.

## Qué hace

**Escuchar**

- Biblioteca por referencia (MP3, FLAC, WAV, AAC/ALAC, AIFF, Ogg Vorbis), búsqueda, favoritos y playlists en SQLite.
- Cola nativa con repetición; la siguiente pista se decodifica antes de que termine la actual.
- Vista mini siempre encima, visual procedural integrado y, en macOS, un paisaje Metal en su propio proceso.

**Mezclar**

- Consola distribuida como tu teclado: deck A a la izquierda, mixer al centro, deck B a la derecha, samples abajo.
- BPM y tonalidad estimados al importar, notación americana/Camelot y corrección manual del beatgrid.
- Sync nativo de tempo/fase, cuantizado activo por defecto, loops de 1/2/4/8/16 beats y transposición ±12 semitonos con key lock.
- Dos decks con platos, tempo vertical (±50 %), tres hot cues persistentes, loops IN/OUT y nudge; cuatro samples one-shot.
- Mixer con EQ de tres bandas, filtros LP/HP, echo sincronizado al BPM, faders verticales, medidores LED y crossfader equal-power.
- Waveforms de tres bandas (graves, medios, agudos) estilo club: vista general y vista detallada con playhead central.

**Controlar**

- Atajos físicos remapeables; cada tecla ilumina su control y muestra qué acaba de pasar.
- Ayuda contextual fija en el footer: nombre, función, atajos y consejos expandibles al señalar o enfocar controles.
- Hot cues que saltan y reproducen; controles A+B para iniciar, pausar o disparar cue 1 simultáneamente en el motor.
- MIDI Learn para Note On y CC absolutos, con mapeos persistentes.
- Menu bar / tray para play, silencio y salir sin abrir la ventana.

**Hacerla tuya**

- Estilos **curado**, **pixel**, **gótico**, **y2k** y **mínimo**, con tu color (inicial `#d66154`), claro/oscuro/sistema, tipografía, textura (dither, suave o plana) y estilo de waveform.

## Teclado

| Acción          | Deck A     | Deck B    |
| --------------- | ---------- | --------- |
| Play / pausa    | Q, Espacio | P         |
| Volver al cue   | A          | L         |
| Hot cues 1/2/3  | 1 / 2 / 3  | 8 / 9 / 0 |
| Guardar cue     | ⇧ + 1/2/3  | ⇧ + 8/9/0 |
| Loop · IN · OUT | W · E · R  | O · I · U |
| Nudge −/+       | D / F      | J / K     |

Samples `Z X C V` · crossfader `← ↓ →` · silencio total `Esc`. Todo es reasignable en Configuración → Teclado. Detalles en [docs/CONTROLS.md](docs/CONTROLS.md).

## Empezar

Requisitos: [Bun](https://bun.sh) 1.4+, Rust estable y las [dependencias de Tauri](https://v2.tauri.app/start/prerequisites/) de tu sistema. En macOS, Xcode completo (el visual Metal se compila con `swiftc`).

```sh
bun install --frozen-lockfile
bun run desktop
```

Generar la app de macOS (firma ad hoc local, sin notarizar):

```sh
bun run desktop:build
```

Para compilar e instalar la última versión local en `~/Applications/curado.app`, registrada para Spotlight y Raycast:

```sh
bun run desktop:install
# Si acabas de compilar:
bun run desktop:install --no-build
```

El instalador comprueba firma e identidad, copia antes de reemplazar y revierte si falla el cambio. No abre la app ni fuerza su cierre. Si la instalada sigue abierta, ciérrala para actualizarla. La indexación del launcher puede tardar.

El resultado queda en `target/release/bundle/macos/curado.app`. Dentro de la app, **sesión de prueba** (barra lateral) genera dos tracks y cuatro samples locales para probar sin música propia.

`bun run dev` abre solo la interfaz en el navegador, sin audio: el motor necesita el proceso de escritorio. Instrucciones por sistema en [docs/PLATFORMS.md](docs/PLATFORMS.md).

## Cómo está hecho

```text
React / WebView ──acciones validadas──▶ Tauri (Rust) ──cola acotada──▶ callback de audio (CPAL)
      ▲                                    │   ▲                              │
      └──── snapshots 20 Hz ◀── telemetría ┘   └── SQLite (biblioteca, cues, waveforms)
MIDI (midir) ──▶ worker 2 ms ──▶ mismo contrato de acciones
```

- El transporte y la posición pertenecen al motor; la UI no inventa el avance.
- Decodificación y análisis de waveform fuera del callback; la memoria de clips se libera en un worker.
- Sin scripts remotos, sin shell ni acceso genérico a archivos desde JavaScript; CSP local estricta.

Más en [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) y las decisiones en [docs/DECISIONS.md](docs/DECISIONS.md).

## Verificar

```sh
bun run check
bun run build
bun run format:check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
bun run probe
```

`probe` mide el tiempo de render offline con seis fuentes y efectos frente al presupuesto de 5,3 ms por bloque de 256 frames a 48 kHz. Resultados y outliers en [docs/STATUS.md](docs/STATUS.md); no mide latencia física ni garantiza ausencia de cortes.

## Hacia dónde va

Streaming para pistas largas, reloj de posición para un playhead fluido, resampler y EQ de calidad, validación de análisis/sync y transposición sobre un corpus real, más vistas (compacta, escenario) y validación de reproducción/cookies del explorador web. La entrada **radio / web** muestra fractos radio y SoundCloud dentro de la ventana principal: su audio se controla en sus players y queda fuera del mixer, FX, sync y Escape nativos; **detener audio web**, cambiar servicio o salir de esa vista cierra el navegador integrado. Prioridades en [docs/ROADMAP.md](docs/ROADMAP.md) y fichas de trabajo en [docs/SESSIONS.md](docs/SESSIONS.md).

## Contribuir

Lee [AGENTS.md](AGENTS.md): resume los invariantes de tiempo real, seguridad y documentación que cualquier cambio debe respetar. Cada cambio actualiza [docs/STATUS.md](docs/STATUS.md) y [docs/CHANGELOG.md](docs/CHANGELOG.md) con evidencia; «compila» no significa «probado».

## Licencia

Pendiente de definir por el autor antes de publicar. Las fuentes incluidas (Jersey 15, Pixelify Sans, Geist Mono, UnifrakturMaguntia, Michroma) se distribuyen bajo SIL Open Font License 1.1.

---

<sub>**English** — curado is a local-first desktop music player and two-deck mixer. A real-time Rust audio engine (CPAL + Symphonia, lock- and allocation-free callback) drives a Tauri 2 + React interface built for the keyboard: club-style three-band waveforms, a console laid out like your hands, persistent hot cues, MIDI learn and themeable styles. Early development, tested on Apple Silicon; see `docs/` (Spanish) for status and architecture.</sub>
