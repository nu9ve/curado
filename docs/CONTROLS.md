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
- Cada deck: título y tiempo restante, waveform completa con cues numerados y loop, **plato** que gira con la posición real (clic = play/pausa), tres hot cues, transporte (play, cue, nudge −/+), loop/IN/OUT y **pitch vertical** en el borde exterior (±50 %, doble clic vuelve a 0).
- Mixer: por canal HI/MID/LOW (unidad al centro), FILTER (paso bajo, OFF a la derecha), ECHO, medidor LED en dB y fader de volumen vertical; master vertical con medidor, botón de silencio y crossfader horizontal con sus teclas ← ↓ →.
- Mapa del teclado: muestra qué tecla controla qué zona (A coral, B complementario, samples ámbar, mixer neutro). Las teclas se iluminan al pulsarlas y se pueden clicar.

El mixer incluye **A + B**: reproducir ambos, pausar ambos y disparar el primer hot cue de ambos. Cada botón envía una sola acción; ambos transportes cambian antes de la primera muestra del mismo bloque de audio. Cada pista conserva su propio cue y velocidad: esto no ajusta BPM ni alinea beats. Los samples no se ven afectados y los controles no cambian el crossfader ni restauran un master a cero.

El encabezado de cada deck distingue **En pausa**, **Fin de pista**, **Reproduciendo**, **Master a cero**, **Canal a cero**, **Fuera del crossfader** y **Error de salida**, según el snapshot. «Reproduciendo» describe el transporte, no garantiza que el archivo tenga señal audible. Los medidores de canal se toman antes del crossfader/master.

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

Un **hot cue salta y reproduce**, incluso desde pausa o fin de pista; repetirlo no alterna pausa. Los tres cues empiezan en 0:00 y muestran su tiempo, incluso antes de guardar uno. Shift + clic en un hot cue guarda posición sin iniciar reproducción. Clic en waveform busca conservando pausa/reproducción; las flechas también buscan al enfocar la waveform (Shift = paso fino). Un salto de cue, seek o nudge fuera del tramo de loop lo desactiva; dentro lo conserva. Con BPM conocido, Loop usa la longitud elegida en beats; sin grid conserva el fallback de hasta cuatro segundos. Q alinea inicio/final y evita loops parciales de cola. Nudge desplaza la posición, no emula la inercia de un jog.

## BPM, tonalidad y mezcla armónica

Al importar se estiman BPM, offset de beat y tonalidad. La biblioteca muestra BPM original y notación americana (C, C♯, Am…) junto a Camelot. Las pistas antiguas se analizan al cargar; abre **beatgrid / análisis** para corregir BPM (incluye ×2/÷2), marcar «beat aquí», elegir tonalidad o reanalizar. Reanalizar reemplaza correcciones manuales; guardar no modifica el archivo original. La periodicidad mostrada describe el análisis, no un porcentaje de exactitud. Hay música sin un BPM/key global válido.

- **MASTER** elige la referencia. Activa **SYNC** en el otro deck para igualar tempo y fase. No inicia reproducción. Ajusta el BPM desde el master; mover manualmente la velocidad del follower desactiva su sync. El rango de velocidad es 0,5×–1,5× y el campo de tempo absoluto admite 40–300 BPM dentro de ese rango.
- **Q** empieza activo. Cues/IN/OUT y loops usan el beatgrid; el selector crea 1/2/4/8/16 beats. Cerca del final recoloca un loop para conservar beats completos; no crea uno si no cabe. Con SYNC+Q, el cue del seguidor espera al siguiente beat del master y muestra «en espera». Pánico o pausa conjunta cancelan ese cue; nunca debe arrancar después del silencio total.
- **Key lock** empieza activo. Los botones −/+ transponen semitonos sin cambiar BPM; el centro vuelve a 0. El tono mostrado corresponde al ajuste aplicado; sin key lock incluye cents debidos a varispeed. **Key sync** iguala la tónica del master o su relativa si difieren los modos mayor/menor; requiere key lock en el master y no convierte escalas menores en mayores. Es puntual, no un seguimiento permanente.
- Los ajustes de tempo/key que necesiten procesamiento muestran **preparando** y conservan el audio anterior hasta terminar. Se puede seguir usando play, cues y mixer; cambiar la petición cancela la anterior. Si falla, aparece reintento. Esta primera implementación prepara clips completos, por lo que el ajuste puede tardar y consumir memoria adicional.
- El mixer añade **HP** (paso alto independiente de LP) y **ECHO · BEATS** (0,25/0,5/0,75/1/2). Echo sigue el BPM de salida; sin análisis usa 120 BPM. Cambiar su división suaviza el tiempo, con posible barrido de tono en las repeticiones.

Al cargar otra pista se reinician tempo/transposición/key lock de ese deck y su sync; se conservan los FX. Cargar el master desactiva el seguimiento del otro deck. Cambiar a player cancela sync/cues pendientes y devuelve el master a A.

La estimación necesita validación en música real. Revisa el grid antes de mezclar; una pista con tempo variable no queda sincronizada por un único BPM. Los loops todavía no tienen crossfade y transposiciones extremas pueden alterar transitorios/timbre.

## Ayuda contextual en el footer

La franja inferior permanece fuera del scroll del contenido. Al pasar el cursor o enfocar un control con Tab muestra su nombre, función y atajos guardados. **Más info** expande detalles y consejos dentro del mismo footer; el contenido principal cede altura y los avisos se recolocan. El fondo usa el tono del deck o el tono general, respetando el tema. Escape dentro de la ayuda expandida la cierra y devuelve el foco al botón, sin enviar pánico.

La ayuda conserva el último control al entrar en su propio panel para poder leerlo. Las acciones de audio se identifican mediante `data-action` (slot/tipo/valor), no por texto parecido; los controles genéricos usan su etiqueta/título y un texto de base. No hay generación automática de consejos: editar `src/lib/hints.ts` para ampliar o corregir textos y `data-help` para asociar otras secciones. La vista mini conserva su tamaño y no incluye esta franja.

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
