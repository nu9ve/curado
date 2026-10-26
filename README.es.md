<p align="center"><img src="docs/assets/logo.svg" width="72" height="72" alt="curado"></p>
<h1 align="center">curado</h1>
<p align="center">Tu música. Dos decks. Espacio para jugar.<br>Un reproductor y mezclador de escritorio que se toca con el teclado.</p>
<p align="center"><a href="README.md">English</a> · <a href="docs/SETUP.md">Instalar con tu agente</a> · <a href="docs/README.md">Documentación</a> · Hecho por <a href="https://nu9ve.xyz">nu9ve</a></p>

Bienvenido. **curado** es un espacio para escuchar tu colección, probar una transición o armar una mezcla. Combina biblioteca, dos decks, mixer y cuatro pads de samples. Tus archivos se quedan donde están: curado no los mueve ni los modifica.

![Mixer de curado con dos decks, waveforms, mixer central y samples](docs/assets/mixer.png)

_Captura de una sesión de desarrollo; no muestra todas las funciones de la compilación más reciente. La interfaz está actualmente en español; la documentación técnica está en inglés._

> **0.1.0 · en desarrollo.** Compilación y pruebas automatizadas locales en Apple Silicon. Faltan validación de hardware en Windows/Linux, streaming de pistas largas y distribución firmada. Revisa el [estado](docs/STATUS.md) y los límites de abajo antes de depender de curado para una sesión.

## Qué puedes hacer

- **Escuchar:** biblioteca, búsqueda, favoritos, playlists, cola con repetición y vista mini siempre encima.
- **Mezclar:** dos decks, tres hot cues por pista, loops por beats, sync de tempo/fase, EQ, filtros LP/HP y echo por beats.
- **Explorar la música:** BPM/key estimados, notación americana y Camelot, beatgrid editable, key lock y transposición ±12 semitonos.
- **Controlar:** teclado reasignable, MIDI Learn, feedback animado y ayuda contextual en el footer.
- **Personalizar:** cinco estilos, tu propio color, claro/oscuro/sistema y variantes de waveform.
- **Abrir radio:** fractos y SoundCloud dentro de la app. Su audio web es independiente del mixer nativo.

Tu biblioteca y mezcla local funcionan sin internet. Los servicios web sí necesitan conexión.

## Dáselo a tu agente

Abre el repo en tu agente de programación y pega esto:

```text
Ayúdame a instalar curado en este equipo. Lee primero AGENTS.md y docs/SETUP.md.
Revisa mi sistema operativo y requisitos, instala desde los lockfiles y ejecuta
las comprobaciones indicadas. En macOS, compila e instala con bun run desktop:install.
No abras la app ni interrumpas una sesión de audio existente. Dime qué pasó las
pruebas y qué queda pendiente de verificar manualmente.
```

La [guía de instalación](docs/SETUP.md) reúne requisitos, diferencias entre sistemas, actualizaciones y problemas habituales. [AGENTS.md](AGENTS.md) explica los contratos que debe conservar cualquier cambio. No necesitas entender el motor de audio para probar la app.

## Instalar por tu cuenta

El flujo disponible aquí compila desde código; no se promete una descarga pública firmada. Necesitas Bun, Rust estable y los [requisitos de tu plataforma](docs/PLATFORMS.md). En Mac: Xcode completo, herramientas de línea de comandos, C++ y libclang.

```sh
git clone https://github.com/nu9ve/curado.git
cd curado
bun install --frozen-lockfile
bun run desktop:install
```

El instalador es **solo para macOS**. Deja `~/Applications/curado.app`, verifica la firma local y la registra para Spotlight/Raycast. No la abre ni cierra una instancia existente. Sal de curado antes de actualizar; el índice del launcher puede tardar. La firma es ad hoc, no una notarización para distribución.

Para desarrollo: `bun run desktop`. `bun run dev` solo muestra la interfaz en el navegador, sin audio nativo. [Windows y Linux](docs/PLATFORMS.md) tienen otros requisitos y aún necesitan validación física.

Después de cambiar código, `bun run desktop:install` recompila y actualiza tu Mac. `--no-build` solo reutiliza un bundle que acabas de compilar y verificar.

## Tu primera mezcla

1. Abre curado y elige **sesión de prueba** en la barra lateral. Genera dos pistas cortas y cuatro samples locales; no descarga música.
2. Cambia a **Mezcla**, reproduce A y B y mueve el crossfader.
3. Prueba un hot cue o un filtro. Señala cualquier control para ver su ayuda en el footer.
4. Importa tu música. Retirar una pista de la biblioteca nunca borra su archivo original.

| Acción                  | Deck A        | Deck B        |
| ----------------------- | ------------- | ------------- |
| Play/pausa              | Q, Espacio    | P             |
| Primer cue y reproducir | A             | L             |
| Hot cues 1/2/3          | 1 / 2 / 3     | 8 / 9 / 0     |
| Guardar cue             | Shift + 1/2/3 | Shift + 8/9/0 |
| Loop / IN / OUT         | W / E / R     | O / I / U     |
| Nudge −/+               | D / F         | J / K         |

Samples: `Z X C V`. Crossfader: `← ↓ →`. Pánico de audio nativo: `Esc` (no detiene audio web). Todo se reasigna en **Configuración → Teclado**. [Controles completos](docs/CONTROLS.md).

## Límites actuales

- Máximo **96 MiB de PCM decodificado por archivo**, unos 4:22 a 48 kHz estéreo. Las pistas de una hora todavía no están soportadas.
- El BPM/key es una estimación. Cambiar tempo/tono con preparación puede tardar y consumir memoria adicional.
- Playhead a 20 Hz, loops sin crossfade y DSP todavía básico.
- Una salida estéreo, sin preescucha independiente por auriculares; recuperación de dispositivos manual.
- El navegador integrado necesita validación real de reproducción/login y layout. Su audio no responde a master, FX, sync ni pánico locales.

Siguientes prioridades en [Roadmap](docs/ROADMAP.md). Para contribuir, empieza en [AGENTS.md](AGENTS.md) y el [mapa de documentación](docs/README.md). Compilar no demuestra calidad auditiva ni estabilidad en directo.

## Licencia y créditos

Hecho por [nu9ve](https://nu9ve.xyz). **La licencia del código propio aún está pendiente.** Publicar el repo no equivale a dar permiso general para reutilizarlo; las dependencias conservan sus licencias.

La recomendación actual es MIT, que permite forks comerciales. Puedes combinar descargas gratis, donaciones y versiones oficiales de paga. Esa recomendación no aplica una licencia: la comparación y decisiones pendientes están en [Licensing](docs/LICENSING.md).
