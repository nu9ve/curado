# Sistemas, dependencias y distribución

## macOS / Apple Silicon

Signalsmith Stretch 0.1.3 añade compilación C++ y bindgen/libclang. Xcode completo proporciona las herramientas en este Mac; si bindgen no encuentra libclang, configurar `LIBCLANG_PATH` hacia la instalación de LLVM/Xcode. Esto es requisito de build, no un permiso ni servicio en runtime.

Bun, Rust estable, Xcode completo con herramientas de línea de comandos y SDK macOS. El build usa `xcrun swiftc` para AppKit/MetalKit. Target mínimo declarado: macOS 12; el helper se compila para la misma arquitectura Rust con deployment target 12. Compatibilidad con versiones antiguas aún no verificada. La máquina actual compila ARM64 nativo.

```sh
bun install --frozen-lockfile
bun run desktop
bun run desktop:build
```

Resultado: `target/release/bundle/macos/curado.app` (el nombre visible del producto es «curado», en minúsculas; el identificador `studio.curado.player` no cambió, así que datos y preferencias se conservan). El script de build firma ad hoc el contenedor local y verifica su resource seal; si existe identidad de publicación configurada, la respeta. Build local ad hoc; no está notarizado para otros equipos. Para distribuir: Developer ID, hardened runtime, helper firmado dentro del bundle, entitlements mínimos y notarización. No añadir permisos de micrófono: la app solo reproduce. No pedir Accessibility/Input Monitoring para los hotkeys enfocados actuales.

Datos: `~/Library/Application Support/studio.curado.player/curado.sqlite` y carpeta `demo/`. Caché del helper: `~/Library/Caches/studio.curado.player/curado visuals.app` (versiones anteriores dejaban `Curado Visuals.app`; puede borrarse a mano, la app no borra cachés antiguas). SQLite puede tener archivos `-wal`/`-shm`: usar backup consistente con SQLite o cerrar la app antes de copiar.

Instalar/actualizar para el usuario, sin sudo ni lanzamiento automático:

```sh
bun run desktop:install
bun run desktop:install --no-build
bun run desktop:install --check
```

Destino: `~/Applications/curado.app`. El script valida firma/identificador, rechaza un destino que sea symlink u otra aplicación, verifica SHA-256 del ejecutable copiado, reemplaza mediante renames con rollback y registra LaunchServices. Si esa app está abierta, pide cerrarla y no interrumpe su audio. Conserva el identificador y todos los datos de usuario. Spotlight/Raycast pueden tardar en refrescar su índice; no se considera probado su lanzamiento por completar el registro. Si sigue abierta otra instancia desde el checkout, cerrarla antes de lanzar la instalada para evitar que single-instance enfoque el proceso anterior.

## Windows

Rust MSVC, Microsoft C++ Build Tools, Windows SDK, WebView2 y Bun. CPAL utiliza WASAPI; este build no habilita ASIO. midir usa WinMM. Tray disponible mediante Tauri. Crear instalador con `bun tauri build` desde Windows; `desktop:build` está orientado al bundle de Mac y usa `--bundles app`.

El build de Signalsmith requiere LLVM/libclang para bindgen y el compilador C++; su configuración y ejecución física en Windows siguen pendientes de CI/hardware. El instalador local actual es solo macOS.

Pruebas físicas pendientes: shared audio con interfaces/USB/Bluetooth, suspensión, cambio de dispositivo, DPI y layouts de teclado. No afirmar soporte de ASIO por usar CPAL: requiere feature/SDK/build y verificación independiente.

## Linux

Bun, Rust, compilador C y dependencias Tauri/GTK/WebKit más ALSA. En Ubuntu 24.04:

```sh
sudo apt-get update
sudo apt-get install build-essential pkg-config libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev libasound2-dev libclang-dev patchelf
bun install --frozen-lockfile
bun run desktop
```

Este build usa ALSA; la integración con PipeWire/Pulse depende de la configuración del sistema. No se habilitan JACK/PipeWire nativo en CPAL 0.16. midir usa ALSA MIDI. El tray depende del escritorio/extensiones (especialmente GNOME). Wayland puede restringir global shortcuts; aquí los atajos son enfocados. Si no hay tray usable, mostrar una opción explícita para salir/cerrar es prioridad de validación, porque cerrar ventana está configurado para ocultarla si se creó tray.

El visual integrado funciona como UI compartida. El botón Metal solo se muestra en macOS; falta un renderer nativo multiplataforma (wgpu).

## Validación cruzada

CI está preparada para macOS, Windows y Ubuntu. Crear el workflow no demuestra que haya corrido. Una compilación de CI tampoco valida drivers, hardware, hotplug ni tray. Registrar ejecuciones y matriz de hardware en STATUS/QUALITY.

Referencia de requisitos generales: [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/). La aplicación no instala drivers ni servicios al sistema.

## Iconos

`scripts/icon.py` genera el icono de 1024 px y `docs/assets/logo.svg` desde la misma geometría que `src/components/Logo.tsx` (anillo pixel en «c» con eje, dither coral → ámbar sobre squircle oscuro). Después `bun tauri icon <png>` regenera `src-tauri/icons` para macOS, Windows, Linux, iOS y Android. Sin dependencias externas.

## Fuentes

Las fuentes viajan dentro del bundle (`@fontsource/*` 5.3.0, latín, OFL 1.1). No se cargan fuentes de red. Al distribuir, incluir los avisos de licencia de cada familia.
