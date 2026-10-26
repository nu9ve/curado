# Curado

Un reproductor local y estudio de mezcla con audio nativo en Rust, Tauri 2 y React. Diseñado para escuchar, explorar una biblioteca y mezclar con teclado. Primera versión funcional para pruebas locales; los requisitos de distribución y uso en directo están en [calidad](docs/QUALITY.md).

## Probar en esta Mac

La aplicación compilada está en `target/release/bundle/macos/Curado.app`. Ábrela en Finder, pulsa **Cargar sesión de prueba** y después **Play**. Cambia a **Mezcla** para usar los dos decks y Z/X/C/V para samples. El botón **Abrir Metal · macOS** abre un paisaje procedural nativo que recibe audio del master.

Para desarrollar:

```sh
bun install --frozen-lockfile
bun run desktop
```

Para generar la aplicación:

```sh
bun run desktop:build
```

`bun run dev` abre únicamente la interfaz web, sin reproducción. El motor necesita el proceso de escritorio. Ver [requisitos por OS](docs/PLATFORMS.md).

## Incluye

- Archivos mono/estéreo, metadatos, búsqueda, favoritos, playlists SQLite y cola de reproducción nativa con repetición.
- Dos decks, seek, tres hot cues por deck, loops IN/OUT, varispeed y nudge.
- EQ de tres bandas, filtro paso bajo, echo, crossfader equal power, cuatro samples y master protegido por un techo de amplitud.
- Temas claro/oscuro/sistema, variantes y color HSV/hex configurable (inicial `#d66154`). Cabecera macOS integrada sin franja separada de título.
- Atajos visibles y reconfigurables; MIDI Learn para Note On y CC absolutos.
- Menu bar/tray con mostrar, play/pausa, silenciar y salir. Cerrar la ventana conserva el proceso y el audio.
- Visual integrado limitado a 30 FPS y ventana AppKit/MetalKit independiente en macOS.
- Controles de dispositivo/buffer y recuperación manual de salida.

## Límites actuales

96 MiB de PCM por archivo (unos 4:22 a 48 kHz estéreo); sin streaming todavía. Velocidad y tono están unidos. No hay beat sync, key lock, preescucha multicanal, video ni plugins. Los cues se guardan por pista; loops y sesión de decks aún son temporales. El avance de cola puede tener un intervalo al decodificar la siguiente pista. macOS fue comprobado localmente; Windows/Linux tienen implementación compartida y CI configurada, pendiente de ejecución y validación física.

La UI usa WKWebView en macOS; el audio usa CoreAudio. El visualizador Metal sí es nativo. No se promete latencia de escenario ni tiempo real duro.

## Continuar

Empezar en [docs/README.md](docs/README.md) y [estado actual](docs/STATUS.md). Las sesiones deben actualizar esos documentos y el [registro](docs/CHANGELOG.md); `AGENTS.md` recoge los invariantes.
