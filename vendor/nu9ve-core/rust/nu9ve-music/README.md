# nu9ve-music

Análisis musical sin conexión de una pista entera:
- **una grilla constante:** BPM, primer beat (`beat_offset`), inicio de compás (`downbeat`) y cuán clara es (`bpm_confidence`);
- **un mapa de tempo**, para la música que se mueve (`follow_tempo`), sólo a pedido;
- **el primer sonido audible**;
- **la clave**, con su confianza.

Es determinista y acotado, y nunca corre en el callback de audio. Sólo depende de `serde`, porque las apps guardan `MusicAnalysis` como JSON. Trae su propia FFT.

```rust
let analysis = nu9ve_music::analyze(&stereo, 48_000); // &[[f32; 2]]
analysis.bpm;        // Option<f64>
analysis.downbeat;   // segundos de un primer tiempo
analysis.key;        // Option<MusicalKey { tonic: 0..12, minor }>
```

## De dónde sale

De curado (`crates/audio-engine/src/beat.rs` y `music.rs`), leído el 2026-11-22. Se mudó tal cual, con sus 13 pruebas. La otra implementación de la suite, la de SoundBender (`SoundCore`, `Tempo.swift` y `ChromaAnalysis.swift`), se comparó antes de elegir, con las mismas señales:

| Señal | Real | curado (este crate) | SoundBender |
| --- | --- | --- | --- |
| Bombo en negras, 120 | 120 | **120,01** | 60 (mitad) |
| Bombo en negras, 128 | 128 | **128,0** | 127,73 |
| Bombo en negras, 140 | 140 | **140,0** | 139,63 |
| Bombo en negras, 100, empieza a 0,37 s | 100 | **100,01**, primer beat 0,371 | 50 (mitad), primer beat 0,37 |
| Hip-hop, 90 | 90 | **90,0** | 90,09 |
| Hip-hop, 70 | 70 | 140 (doble) | **70,05** |
| Drum and bass, 174 | 174 | 130,5 | 86,97 (mitad) |
| Ruido | sin pulso | **sin pulso** | 119 (falso) |
| Seis progresiones I–IV–V–I, mayores y menores | la clave | **6 de 6**, confianza 0,64–1,0 | 6 de 6, confianza 0,03–0,16 |

curado gana en el tempo, en el primer beat y en rechazar lo que no tiene pulso. En la clave empatan, pero curado lo dice con mucha más confianza. Por eso el crate parte de curado.

## Límites conocidos

Están en los vectores ([`data/music/music-vectors-v1.json`](../../data/music/music-vectors-v1.json), campo `known`), con lo que responde hoy cada uno, para que un arreglo se vea como un cambio ahí:

- **Inicio de compás un tiempo tarde:** con el bombo acentuado en el primer tiempo, a 128, 140 y en la señal que empieza tarde. A 120 acierta.
- **Hip-hop lento al doble:** 70 se lee como 140.
- **Drum and bass:** 174 se lee como 130,5, ni el tempo ni su mitad.

Son señales sintéticas. Ninguno de estos límites se vio todavía con música real, que es para lo que curado lo afinó (su biblioteca de pistas de club).

## Pruebas

```bash
cargo test -p nu9ve-music
```

Las 13 de curado, y `tests/vectors.rs`, que sintetiza las señales de los vectores (con ruido xorshift64* sembrado por señal, el mismo en cualquier lenguaje) y compara con la verdad, o con el fallo conocido. `NU9VE_PRINT_MUSIC=1` imprime lo que responde.

## Quién lo usa

Nadie todavía. curado es la app de origen: cambia su `beat.rs` y su `music.rs` por este crate en un cambio propio. SoundBender lo usaría desde su motor en Rust (`soundbender-core`) en lugar de `TempoAnalysis` y `ChromaAnalysis`, a través de `soundbender-ffi`. Clipper analiza en Python. [Adopción por app](../../docs/adoption.md#nu9ve-music).
