# Apariencia y ventana nativa

El color inicial exacto es **#d66154**. No se cambia el color elegido para compensar contraste: `--accent` conserva ese hexadecimal y `--accent-text` deriva un tono legible para texto. `--accent-ink` elige texto claro/oscuro sobre botones. El deck B tiene un acento complementario y su propia tinta legible.

## Controles

Configuración → **Apariencia** abre:

- Oscuro, claro o seguir al sistema operativo.
- Variación neutra, tonal (tiñe superficies) o de mayor contraste.
- Presets coral, ámbar, salvia, océano, lavanda y rosa.
- Selector de color, entrada hexadecimal y sliders HSV (tono, saturación y brillo).
- Restaurar el coral y esquema inicial.

Cambiar el tema no reconstruye el motor, no pausa decks ni modifica efectos o routing. Los ajustes se muestran inmediatamente; SQLite guarda el último valor con debounce de 180 ms. Un caché local evita esperar a SQLite en la primera pintura. El cambio de sistema solo se sigue si se seleccionó Sistema. La configuración inválida o de versión desconocida vuelve al valor inicial.

Los tokens actualizan superficies, texto, bordes, botones, sliders, medidores, waveform y color del paisaje. Metal recibe el acento por la telemetría; su escena mantiene un fondo oscuro. La actualización de ventana se agrupa y no participa en el callback de audio.

## Implementación

- `src/lib/theme.ts`: matemática RGB/HSV, validación, presets, luminancia/contraste y paleta pura.
- `src/lib/theme-store.ts`: caché, preferencia del OS, actualización de tokens, persistencia serializada y sincronización con ventana.
- `src/components/ThemePanel.tsx`: controles, HSV con preservación de tono en negro/grises y vista de muestra integrada.
- `scripts/sync-theme.ts`: genera los valores iniciales de CSS a partir de la misma paleta; corre en cada build.
- `src-tauri/src/preferences.rs`: validación de preferencias no confiables y tipo serializable.

Añadir colores mediante tokens semánticos, evitando hexadecimales en componentes. Los colores de presets, de estados semánticos y del generador de paleta sí son constantes intencionales. Las pruebas detectan tokens no definidos, divergencias entre paleta/CSS inicial y retorno al antiguo verde.

## Cabecera macOS

`tauri.macos.conf.json` usa `titleBarStyle: Overlay`, `hiddenTitle: true` y controles nativos reposicionados. El contenido llega a la parte superior: se elimina la franja separada de título. Se conservan los controles estándar de cerrar/minimizar/fullscreen. Una zona de arrastre sobre la sidebar y partes libres de la cabecera permite mover la ventana; botones e inputs conservan su interacción.

Esta configuración es específica de macOS. Windows/Linux conservan sus decoraciones nativas. No se modifica la barra global de menús de macOS ni preferencias del sistema. Solo se conceden permisos Tauri para arrastrar ventana y ajustar su tema/fondo.

## Evidencia y límites

Pruebas puras cubren 162 combinaciones de base/mode/variante/estado del OS, extremos negro/blanco, round trip HSV, hex inválido, contraste de texto y tinta. Contraste se calcula matemáticamente: no sustituye una auditoría de toda la interfaz con lector de pantalla o revisión humana.

Por instrucción del usuario, esta entrega se verifica con código, compilación y tests, sin Computer Use ni revisión visual posterior al cambio de tema/cabecera. Registrar esa diferencia en STATUS; no afirmar que hubo QA visual de estos cambios.
