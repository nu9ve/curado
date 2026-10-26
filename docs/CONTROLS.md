# Uso y controles

## Primera sesión

1. Abrir `target/release/bundle/macos/curado.app`.
2. Elegir **sesión de prueba** (abajo en la barra lateral): genera dos tracks de 32 segundos y cuatro samples locales en el directorio de datos, registra todo y carga los seis slots. No descarga música.
3. Pulsar Play en Reproductor. Para tu música, usar **Importar música** o **Carpeta**.
4. Seleccionar una fila y cargar en A/B. Doble clic carga A; después Play. La carga detiene/reinicia solo el slot de destino.
5. Crear playlist con + lateral (doble clic sobre su nombre para renombrar), seleccionar pista y usar «Añadir a playlist». «Escuchar selección» recorre las pistas filtradas en orden, desde la primera. El selector permite no repetir, repetir todo o repetir pista.
6. Cambiar a Mezcla para acceder a la consola. Seleccionar una pista y pulsar + en un pad para asignarla como sample.
7. En reproductor, «Metal» (solo macOS) abre un paisaje nativo; el botón del ojo pausa el visual integrado.
8. El botón de imagen en imagen de la topbar abre la **vista mini** siempre encima; el botón de expandir vuelve a la vista completa con el tamaño anterior.

## Consola de mezcla

La vista Mezcla se distribuye como el teclado: **deck A a la izquierda, deck B a la derecha, mixer al centro, samples abajo**.

- Arriba, dos waveforms detalladas (A y B) con el playhead fijo al centro y 8 s visibles; clic para buscar relativo al centro. Graves en azul, medios ámbar, agudos blanco (o RGB/mono según Apariencia).
- Cada deck: título y tiempo restante, waveform completa con cues numerados y loop, **plato** que gira con la posición real (clic = play/pausa), tres hot cues, transporte (play, cue, nudge −/+), loop/IN/OUT y **pitch vertical** en el borde exterior (±20 %, doble clic vuelve a 0).
- Mixer: por canal HI/MID/LOW (unidad al centro), FILTER (paso bajo, OFF a la derecha), ECHO, medidor LED en dB y fader de volumen vertical; master vertical con medidor, botón de silencio y crossfader horizontal con sus teclas ← ↓ →.
- Mapa del teclado: muestra qué tecla controla qué zona (A coral, B complementario, samples ámbar, mixer neutro). Las teclas se iluminan al pulsarlas y se pueden clicar.

Perillas y faders: arrastrar vertical (Shift = fino), rueda, flechas (Shift = fino, RePág/AvPág = grueso), Inicio = valor por defecto, doble clic = restaurar. Mientras arrastras, el control muestra tu valor local y después vuelve a seguir al motor.

Cada atajo de teclado ilumina el control correspondiente y muestra un **HUD** con la tecla y la acción (p. ej. «A · Hot cue 2»). Si el modo actual bloquea la acción, el HUD lo dice («no disponible») y no se envía nada al motor.

## Teclado por defecto

| Acción                         | Deck A        | Deck B        |
| ------------------------------ | ------------- | ------------- |
| Play / pausa                   | Q, Espacio    | P             |
| Volver al primer cue           | A             | L             |
| Hot cues 1/2/3                 | 1 / 2 / 3     | 8 / 9 / 0     |
| Guardar cue en posición actual | Shift + 1/2/3 | Shift + 8/9/0 |
| Loop on/off                    | W             | O             |
| Loop IN                        | E             | I             |
| Loop OUT                       | R             | U             |
| Nudge −50/+50 ms               | D / F         | J / K         |

| General                                 | Teclas        |
| --------------------------------------- | ------------- |
| Samples 1–4                             | Z / X / C / V |
| Crossfader izquierda / centro / derecha | ← / ↓ / →     |
| Detener decks y bajar master a cero     | ESC           |

Shift + clic en un hot cue también guarda posición. Clic en waveform busca. Loop sin puntos crea un tramo de hasta cuatro segundos desde la posición actual; no está ajustado a beats. Nudge desplaza la posición, no emula la inercia de un jog.

Los atajos se activan con la ventana enfocada, se basan en códigos físicos y no se ejecutan al escribir en inputs o mover sliders. Space/Enter sobre un botón conservan su activación accesible. Las etiquetas muestran el mapeo guardado; Configuración → Teclado permite reasignar y detecta conflictos. Player desactiva hotkeys del deck B y samples. Pánico requiere después subir master y reanudar reproducción.

## MIDI Learn

Configuración → MIDI → seleccionar puerto → elegir acción → Aprender → pulsar nota o mover CC. Se guarda dispositivo, canal, tipo y número; aprender otra acción sobre el mismo control sustituye la anterior. La conexión es manual en cada arranque. Los mapeos persisten. Los cues se guardan por pista y se comparten si la cargas en ambos decks.

Admite Note On con velocity >0 y CC absolutos 0–127; en EQ, CC 64 equivale a ganancia unitaria. No admite todavía encoders relativos, Clock, SysEx, feedback LED ni reconexión automática. Un mapeo CC hacia botón dispara cada mensaje ≥64; para botones preferir Note On. En player las acciones de reproducción de B y samples también se bloquean desde MIDI; cambiar a Mezcla para utilizarlas.

## Audio y segundo plano

Configuración → Audio muestra backend, sample rate, callbacks y muestras recortadas al techo. Un contador de recorte creciente indica exceso de nivel; bajar ganancia/master. Cambiar salida/buffer descarga los seis slots. Buffer automático es el punto inicial; probar 256/512 solo con mediciones. Bluetooth agrega latencia que esta app no elimina.

Cerrar ventana la oculta si existe tray/menu bar. Usar el icono de curado para mostrarla, play/pausa A, silenciar o salir. La reproducción no sobrevive a salir del proceso ni a suspender el sistema. No hay daemon ni servicio instalado.

Si desconectas la interfaz y se reporta error, elegir una salida disponible y aplicar/recuperar audio. Reabrir la UI no reconstruye el motor. Los originales permanecen en sus rutas; si se movieron, reimportarlos.

## Apariencia

Configuración → Apariencia: estilo (curado, pixel, gótico, y2k, mínimo), claro/oscuro/sistema, variación neutra/tonal/contraste, tipografía, textura (dither/suave/plano), estilo de waveform y color por presets, selector, hex o HSV. El valor inicial es estilo curado con `#d66154`. Se guarda automáticamente; Escape cierra Configuración. La cabecera de macOS integra controles nativos y zona de arrastre sin franja de título separada. Ver [THEMES](THEMES.md).
