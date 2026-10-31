//! Read-only reader for rekordbox device exports: `export.pdb` and `ANLZ` analysis files.
//!
//! Formats are reverse engineered, documented by Deep Symmetry's Crate Digger. A USB stick is
//! untrusted input, so every read is bounds-checked and a malformed file gives an error or a
//! shorter result, never a panic. Nothing here writes to a device.
use curado_audio::music::MusicalKey;
use serde::Serialize;

const TABLE_TRACKS: u32 = 0x00;
const TABLE_GENRES: u32 = 0x01;
const TABLE_ARTISTS: u32 = 0x02;
const TABLE_ALBUMS: u32 = 0x03;
const TABLE_KEYS: u32 = 0x05;
const TABLE_PLAYLIST_TREE: u32 = 0x07;
const TABLE_PLAYLIST_ENTRIES: u32 = 0x08;
const TABLE_ARTWORK: u32 = 0x0d;
/// Rows sit after the page's fixed header, and their offsets count from here.
const PAGE_HEAP: usize = 0x28;
/// One group: 16 row offsets, a presence mask and a transaction mask.
const ROW_GROUP: usize = 16 * 2 + 2 + 2;
const INDEX_PAGE_FLAG: u8 = 0x40;
const STRING_ANALYSIS_PATH: usize = 14;
const STRING_TITLE: usize = 17;
const STRING_FILE_PATH: usize = 20;
/// Hot cues A to H.
pub const HOT_CUES: usize = 8;

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DeviceTrack {
    pub id: u32,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub genre: String,
    pub key: Option<String>,
    pub bpm: f64,
    pub duration: u32,
    pub rating: u8,
    /// Relative to the device root. Stays native: the interface only receives track ids.
    #[serde(skip)]
    pub path: String,
    #[serde(skip)]
    pub analysis_path: String,
    /// The picture rekordbox made for it, relative to the device root. Empty without one.
    #[serde(skip)]
    pub artwork: String,
}
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistNode {
    pub id: u32,
    pub parent: u32,
    pub name: String,
    pub folder: bool,
    pub sort: u32,
    pub tracks: Vec<u32>,
}
#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct Export {
    pub tracks: Vec<DeviceTrack>,
    pub playlists: Vec<PlaylistNode>,
}

fn u16_le(data: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(data.get(at..at + 2)?.try_into().ok()?))
}
fn u32_le(data: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(data.get(at..at + 4)?.try_into().ok()?))
}
fn u16_be(data: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes(data.get(at..at + 2)?.try_into().ok()?))
}
fn u32_be(data: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(data.get(at..at + 4)?.try_into().ok()?))
}
fn utf16(raw: &[u8], decode: fn([u8; 2]) -> u16) -> String {
    let units: Vec<u16> = raw.as_chunks::<2>().0.iter().map(|c| decode(*c)).collect();
    String::from_utf16_lossy(&units)
        .trim_end_matches('\0')
        .to_string()
}
/// A DeviceSQL string: short ASCII, long ASCII or long UTF-16.
fn device_string(row: &[u8], at: usize) -> Option<String> {
    let flags = *row.get(at)?;
    if flags & 1 == 1 {
        let length = (flags >> 1) as usize;
        let text = row.get(at + 1..at + length.max(1))?;
        return Some(String::from_utf8_lossy(text).to_string());
    }
    let length = u16_le(row, at + 1)? as usize;
    let text = row.get(at + 4..at + length.max(4))?;
    Some(match flags {
        0x90 => utf16(text, u16::from_le_bytes),
        _ => String::from_utf8_lossy(text).to_string(),
    })
}

/// Calls `visit` with every live row of a table, each slice running to the end of its page.
fn each_row(data: &[u8], table: u32, mut visit: impl FnMut(&[u8])) -> Result<(), String> {
    let invalid = || String::from("La base de datos del dispositivo no es válida");
    let page_size = u32_le(data, 0x04).ok_or_else(invalid)? as usize;
    let tables = u32_le(data, 0x08).ok_or_else(invalid)? as usize;
    if !(512..=1 << 20).contains(&page_size) || tables > 64 {
        return Err(invalid());
    }
    let pointer = (0..tables)
        .map(|i| 0x1c + i * 16)
        .find(|at| u32_le(data, *at) == Some(table));
    let Some(pointer) = pointer else {
        return Ok(());
    };
    let mut page = u32_le(data, pointer + 8).ok_or_else(invalid)? as usize;
    let last = u32_le(data, pointer + 12).ok_or_else(invalid)? as usize;
    // A corrupt chain could loop forever; a real one never visits more pages than exist.
    for _ in 0..data.len() / page_size + 1 {
        let Some(bytes) = page
            .checked_mul(page_size)
            .and_then(|start| data.get(start..start + page_size))
        else {
            return Ok(());
        };
        let flags = bytes[0x1b];
        if u32_le(bytes, 0x08) == Some(table) && flags & INDEX_PAGE_FLAG == 0 {
            let packed = u32::from_le_bytes([bytes[0x18], bytes[0x19], bytes[0x1a], 0]);
            let offsets = (packed & 0x1fff) as usize;
            for group in 0..offsets.div_ceil(16) {
                let Some(end) = page_size.checked_sub(group * ROW_GROUP) else {
                    break;
                };
                let Some(present) = end.checked_sub(4).and_then(|at| u16_le(bytes, at)) else {
                    break;
                };
                for row in 0..16.min(offsets - group * 16) {
                    if present & (1 << row) == 0 {
                        continue;
                    }
                    let offset = end
                        .checked_sub(6 + row * 2)
                        .and_then(|at| u16_le(bytes, at));
                    if let Some(row) = offset.and_then(|at| bytes.get(PAGE_HEAP + at as usize..)) {
                        visit(row);
                    }
                }
            }
        }
        if page == last {
            break;
        }
        page = u32_le(bytes, 0x0c).ok_or_else(invalid)? as usize;
    }
    Ok(())
}
fn names(data: &[u8], table: u32, row: fn(&[u8]) -> Option<(u32, String)>) -> Vec<(u32, String)> {
    let mut found = Vec::new();
    let _ = each_row(data, table, |bytes| found.extend(row(bytes)));
    found.sort_unstable_by_key(|entry| entry.0);
    found
}
fn lookup(table: &[(u32, String)], id: u32) -> Option<&str> {
    let index = table.binary_search_by_key(&id, |entry| entry.0).ok()?;
    Some(table[index].1.as_str())
}

pub fn parse_export(data: &[u8]) -> Result<Export, String> {
    let artists = names(data, TABLE_ARTISTS, |row| {
        let id = u32_le(row, 0x04)?;
        let at = match u16_le(row, 0)? {
            0x64 => u16_le(row, 0x0a)? as usize,
            _ => *row.get(0x09)? as usize,
        };
        Some((id, device_string(row, at)?))
    });
    let albums = names(data, TABLE_ALBUMS, |row| {
        let id = u32_le(row, 0x0c)?;
        let at = match u16_le(row, 0)? {
            0x84 => u16_le(row, 0x16)? as usize,
            _ => *row.get(0x15)? as usize,
        };
        Some((id, device_string(row, at)?))
    });
    let genres = names(data, TABLE_GENRES, |row| {
        Some((u32_le(row, 0)?, device_string(row, 4)?))
    });
    let keys = names(data, TABLE_KEYS, |row| {
        Some((u32_le(row, 0)?, device_string(row, 8)?))
    });
    let artwork = names(data, TABLE_ARTWORK, |row| {
        Some((u32_le(row, 0)?, device_string(row, 4)?))
    });
    let mut export = Export::default();
    each_row(data, TABLE_TRACKS, |row| {
        let string = |index: usize| -> Option<String> {
            device_string(row, u16_le(row, 0x5e + index * 2)? as usize)
        };
        let track = (|| {
            Some(DeviceTrack {
                id: u32_le(row, 0x48)?,
                title: string(STRING_TITLE)?,
                artist: lookup(&artists, u32_le(row, 0x44)?)
                    .unwrap_or_default()
                    .into(),
                album: lookup(&albums, u32_le(row, 0x40)?)
                    .unwrap_or_default()
                    .into(),
                genre: lookup(&genres, u32_le(row, 0x3c)?)
                    .unwrap_or_default()
                    .into(),
                key: lookup(&keys, u32_le(row, 0x20)?).map(str::to_string),
                bpm: u32_le(row, 0x38)? as f64 / 100.,
                duration: u16_le(row, 0x54)? as u32,
                rating: *row.get(0x59)?,
                path: string(STRING_FILE_PATH)?,
                analysis_path: string(STRING_ANALYSIS_PATH)?,
                artwork: lookup(&artwork, u32_le(row, 0x1c)?)
                    .unwrap_or_default()
                    .into(),
            })
        })();
        export.tracks.extend(track);
    })?;
    each_row(data, TABLE_PLAYLIST_TREE, |row| {
        let node = (|| {
            Some(PlaylistNode {
                parent: u32_le(row, 0x00)?,
                sort: u32_le(row, 0x08)?,
                id: u32_le(row, 0x0c)?,
                folder: u32_le(row, 0x10)? != 0,
                name: device_string(row, 0x14)?,
                tracks: Vec::new(),
            })
        })();
        export.playlists.extend(node);
    })?;
    let mut entries = Vec::new();
    each_row(data, TABLE_PLAYLIST_ENTRIES, |row| {
        if let (Some(index), Some(track), Some(playlist)) =
            (u32_le(row, 0), u32_le(row, 4), u32_le(row, 8))
        {
            entries.push((playlist, index, track));
        }
    })?;
    entries.sort_unstable();
    for (playlist, _, track) in entries {
        if let Some(node) = export.playlists.iter_mut().find(|n| n.id == playlist) {
            node.tracks.push(track);
        }
    }
    export.tracks.sort_unstable_by_key(|track| track.id);
    export.tracks.dedup_by_key(|track| track.id);
    Ok(export)
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Cue {
    /// Seconds. Loops also have an end.
    pub start: f64,
    pub end: Option<f64>,
    pub color: Option<[u8; 3]>,
    pub label: Option<String>,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Analysis {
    /// `(beat within bar, tempo × 100, milliseconds)` per grid line.
    pub beats: Vec<(u16, u16, u32)>,
    pub hot: [Option<Cue>; HOT_CUES],
    pub memory: Vec<Cue>,
}
/// An unused position in rekordbox's cue lists.
const NO_TIME: u32 = 0xffff_ffff;

fn point_cue(start: u32, kind: u8, end: u32) -> Option<Cue> {
    (start != NO_TIME).then(|| Cue {
        start: start as f64 / 1000.,
        end: (kind == 2 && end != NO_TIME && end > start).then_some(end as f64 / 1000.),
        color: None,
        label: None,
    })
}
fn sections(data: &[u8]) -> Vec<(&[u8], usize)> {
    let mut found = Vec::new();
    if data.get(..4) != Some(b"PMAI") {
        return found;
    }
    let mut at = u32_be(data, 4).unwrap_or(0) as usize;
    while let (Some(section), Some(length)) = (data.get(at..), u32_be(data, at + 8)) {
        let length = length as usize;
        let Some(section) = section.get(..length).filter(|_| length >= 12) else {
            break;
        };
        found.push((section, u32_be(section, 4).unwrap_or(0) as usize));
        at += length;
    }
    found
}
/// `.DAT` holds the grid and basic cues. `.EXT` repeats the cues with colors and comments.
pub fn parse_analysis(dat: &[u8], ext: Option<&[u8]>) -> Analysis {
    let mut analysis = Analysis::default();
    let mut basic_hot = false;
    for (section, header) in sections(dat) {
        match section.get(..4) {
            Some(b"PQTZ") => {
                let count = u32_be(section, 0x14).unwrap_or(0) as usize;
                let beats = section.get(header..).unwrap_or_default().as_chunks::<8>().0;
                analysis.beats = beats
                    .iter()
                    .take(count)
                    .map(|b| {
                        (
                            u16::from_be_bytes([b[0], b[1]]),
                            u16::from_be_bytes([b[2], b[3]]),
                            u32::from_be_bytes([b[4], b[5], b[6], b[7]]),
                        )
                    })
                    .collect();
            }
            Some(b"PCOB") => {
                let mut at = header;
                for _ in 0..u16_be(section, 0x12).unwrap_or(0) {
                    let Some(length) = u32_be(section, at + 8).map(|v| v as usize) else {
                        break;
                    };
                    let cue = (|| {
                        let hot = u32_be(section, at + 0x0c)? as usize;
                        let cue = point_cue(
                            u32_be(section, at + 0x20)?,
                            *section.get(at + 0x1c)?,
                            u32_be(section, at + 0x24)?,
                        )?;
                        Some((hot, cue))
                    })();
                    if let Some((hot, cue)) = cue {
                        if hot == 0 {
                            analysis.memory.push(cue);
                        } else if let Some(slot) = analysis.hot.get_mut(hot - 1) {
                            *slot = Some(cue);
                            basic_hot = true;
                        }
                    }
                    if length == 0 {
                        break;
                    }
                    at += length;
                }
            }
            _ => {}
        }
    }
    let (mut hot, mut memory) = (Vec::new(), Vec::new());
    for (section, header) in ext.map(sections).unwrap_or_default() {
        if section.get(..4) != Some(b"PCO2") {
            continue;
        }
        let mut at = header;
        for _ in 0..u16_be(section, 0x10).unwrap_or(0) {
            let Some(length) = u32_be(section, at + 8).map(|v| v as usize) else {
                break;
            };
            let Some(entry) = section.get(at..at + length).filter(|_| length >= 0x1c) else {
                break;
            };
            if let Some((index, cue)) = extended_cue(entry) {
                if index == 0 {
                    memory.push(cue);
                } else {
                    hot.push((index, cue));
                }
            }
            at += length;
        }
    }
    // The extended list is richer, but some exports only have the basic one.
    if !hot.is_empty() || !basic_hot {
        analysis.hot = Default::default();
        for (index, cue) in hot {
            if let Some(slot) = analysis.hot.get_mut(index - 1) {
                *slot = Some(cue);
            }
        }
    }
    if !memory.is_empty() {
        analysis.memory = memory;
    }
    analysis.memory.sort_by(|a, b| a.start.total_cmp(&b.start));
    analysis
}
fn extended_cue(entry: &[u8]) -> Option<(usize, Cue)> {
    let index = u32_be(entry, 0x0c)? as usize;
    let mut cue = point_cue(
        u32_be(entry, 0x14)?,
        *entry.get(0x10)?,
        u32_be(entry, 0x18)?,
    )?;
    // Some entries end before the comment or the color, which are optional.
    if let Some(length) = u32_be(entry, 0x28).map(|v| v as usize) {
        let label = entry
            .get(0x2c..0x2c + length)
            .map(|raw| utf16(raw, u16::from_be_bytes))
            .filter(|text| !text.is_empty());
        cue.label = label;
        if let Some(color) = entry.get(0x2c + length..0x2c + length + 4) {
            if color[0] != 0 || color[1..] != [0, 0, 0] {
                cue.color = Some([color[1], color[2], color[3]]);
            }
        }
    }
    Some((index, cue))
}

/// Every beat in seconds, when the stored grid has more than one tempo. A spread this small is
/// rounding: rekordbox writes tempos with two decimals.
pub fn variable_beats(beats: &[(u16, u16, u32)]) -> Option<Vec<f64>> {
    let lowest = beats.iter().map(|beat| beat.1).min()?;
    let highest = beats.iter().map(|beat| beat.1).max()?;
    (highest - lowest > 10 && beats.len() >= 2).then(|| {
        let mut times: Vec<f64> = beats.iter().map(|beat| beat.2 as f64 / 1000.).collect();
        times.dedup();
        times
    })
}

/// One BPM, grid start and bar start. A grid with several tempos keeps its most common one.
pub fn constant_grid(beats: &[(u16, u16, u32)]) -> Option<(f64, f64, f64)> {
    let first = beats.first()?;
    let mut tempos: Vec<u16> = beats.iter().map(|beat| beat.1).collect();
    tempos.sort_unstable();
    let tempo = tempos[tempos.len() / 2];
    if tempo == 0 {
        return None;
    }
    let bpm = tempo as f64 / 100.;
    let period = 60. / bpm;
    let start = first.2 as f64 / 1000.;
    let bar = beats
        .iter()
        .take(8)
        .find(|beat| beat.0 == 1)
        .map_or(start, |beat| beat.2 as f64 / 1000.);
    Some((bpm, start.rem_euclid(period), bar))
}

/// rekordbox stores keys as text, in the notation chosen in its settings.
pub fn parse_key(name: &str) -> Option<MusicalKey> {
    let name = name.trim();
    if let Some((number, letter)) = name
        .strip_suffix(['A', 'B', 'a', 'b'])
        .and_then(|n| n.parse::<i32>().ok())
        .zip(name.chars().last())
        .filter(|(number, _)| (1..=12).contains(number))
    {
        let minor = letter.eq_ignore_ascii_case(&'a');
        // Camelot 8B is C and 8A is A minor; each step up is a fifth.
        let tonic = (number - 8) * 7 + if minor { 9 } else { 0 };
        return Some(MusicalKey {
            tonic: tonic.rem_euclid(12) as u8,
            minor,
        });
    }
    // Open Key: 1d is C and 1m is A minor; each step up is a fifth.
    if let Some((number, minor)) = name
        .strip_suffix(['d', 'm'])
        .and_then(|n| n.parse::<i32>().ok())
        .filter(|number| (1..=12).contains(number))
        .map(|number| (number, name.ends_with('m')))
    {
        let tonic = (number - 1) * 7 + if minor { 9 } else { 0 };
        return Some(MusicalKey {
            tonic: tonic.rem_euclid(12) as u8,
            minor,
        });
    }
    let mut chars = name.chars();
    let base: i32 = match chars.next()?.to_ascii_uppercase() {
        'C' => 0,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        'B' => 11,
        _ => return None,
    };
    let rest = chars.as_str();
    let (shift, mode) = match rest.chars().next() {
        Some('#' | '♯') => (1, &rest[rest.chars().next()?.len_utf8()..]),
        Some('b' | '♭') => (-1, &rest[rest.chars().next()?.len_utf8()..]),
        _ => (0, rest),
    };
    let mode = mode.trim().to_ascii_lowercase();
    let minor = match mode.as_str() {
        "" | "maj" | "major" => false,
        "m" | "min" | "minor" => true,
        _ => return None,
    };
    Some(MusicalKey {
        tonic: (base + shift).rem_euclid(12) as u8,
        minor,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: usize = 8192;
    /// Builds a minimal, valid `export.pdb` so the reader is tested without a real device.
    struct Pdb {
        page_size: usize,
        pages: Vec<Vec<u8>>,
        tables: Vec<(u32, u32, u32)>,
    }
    impl Pdb {
        fn new() -> Self {
            Self {
                page_size: PAGE,
                pages: vec![vec![0; PAGE]],
                tables: Vec::new(),
            }
        }
        /// Each table is a chain of pages; `rows` beyond 16 per page spill into groups.
        fn table(&mut self, kind: u32, rows: &[Vec<u8>], absent: &[usize]) {
            let index = self.pages.len() as u32;
            let mut page = vec![0_u8; self.page_size];
            page[0x08..0x0c].copy_from_slice(&kind.to_le_bytes());
            page[0x0c..0x10].copy_from_slice(&(index + 1).to_le_bytes());
            page[0x1b] = 0x24;
            let packed = (rows.len() as u32) | ((rows.len() - absent.len()) as u32) << 13;
            page[0x18..0x1b].copy_from_slice(&packed.to_le_bytes()[..3]);
            let mut heap = PAGE_HEAP;
            for (i, row) in rows.iter().enumerate() {
                let (group, slot) = (i / 16, i % 16);
                let end = self.page_size - group * ROW_GROUP;
                let at = end - 6 - slot * 2;
                page[at..at + 2].copy_from_slice(&((heap - PAGE_HEAP) as u16).to_le_bytes());
                if !absent.contains(&i) {
                    let mask = u16::from_le_bytes([page[end - 4], page[end - 3]]) | 1 << slot;
                    page[end - 4..end - 2].copy_from_slice(&mask.to_le_bytes());
                }
                page[heap..heap + row.len()].copy_from_slice(row);
                heap += row.len();
            }
            self.pages.push(page);
            self.tables.push((kind, index, index));
        }
        fn bytes(mut self) -> Vec<u8> {
            let page_size = self.page_size as u32;
            let tables = self.tables.len() as u32;
            let first = &mut self.pages[0];
            first[4..8].copy_from_slice(&page_size.to_le_bytes());
            first[8..12].copy_from_slice(&tables.to_le_bytes());
            for (i, (kind, start, end)) in self.tables.iter().enumerate() {
                let at = 0x1c + i * 16;
                first[at..at + 4].copy_from_slice(&kind.to_le_bytes());
                first[at + 8..at + 12].copy_from_slice(&start.to_le_bytes());
                first[at + 12..at + 16].copy_from_slice(&end.to_le_bytes());
            }
            self.pages.concat()
        }
    }
    fn short(text: &str) -> Vec<u8> {
        let mut out = vec![((text.len() as u8 + 1) << 1) | 1];
        out.extend_from_slice(text.as_bytes());
        out
    }
    fn wide(text: &str) -> Vec<u8> {
        let units: Vec<u8> = text.encode_utf16().flat_map(u16::to_le_bytes).collect();
        let mut out = vec![0x90];
        out.extend_from_slice(&(units.len() as u16 + 4).to_le_bytes());
        out.push(0);
        out.extend(units);
        out
    }
    fn track(id: u32, title: &str, artist: u32, key: u32) -> Vec<u8> {
        let mut row = vec![0_u8; 0x88];
        row[0..2].copy_from_slice(&0x24_u16.to_le_bytes());
        row[0x20..0x24].copy_from_slice(&key.to_le_bytes());
        row[0x38..0x3c].copy_from_slice(&12400_u32.to_le_bytes());
        row[0x44..0x48].copy_from_slice(&artist.to_le_bytes());
        row[0x48..0x4c].copy_from_slice(&id.to_le_bytes());
        row[0x54..0x56].copy_from_slice(&215_u16.to_le_bytes());
        row[0x59] = 4;
        let mut place = |index: usize, text: Vec<u8>| {
            let at = row.len() as u16;
            row[0x5e + index * 2..0x60 + index * 2].copy_from_slice(&at.to_le_bytes());
            row.extend(text);
        };
        place(STRING_TITLE, wide(title));
        place(STRING_FILE_PATH, short(&format!("/Contents/{id}.mp3")));
        place(
            STRING_ANALYSIS_PATH,
            short("/PIONEER/USBANLZ/P000/A/ANLZ0000.DAT"),
        );
        row
    }
    fn named(id: u32, name: &[u8]) -> Vec<u8> {
        let mut row = id.to_le_bytes().to_vec();
        row.extend_from_slice(name);
        row
    }
    fn artist(id: u32, name: &str) -> Vec<u8> {
        let mut row = vec![0x60, 0, 0, 0];
        row.extend_from_slice(&id.to_le_bytes());
        row.extend([3, 10]);
        row.extend(short(name));
        row
    }
    fn key(id: u32, name: &str) -> Vec<u8> {
        let mut row = id.to_le_bytes().to_vec();
        row.extend_from_slice(&id.to_le_bytes());
        row.extend(short(name));
        row
    }
    fn node(id: u32, parent: u32, name: &str, folder: bool, sort: u32) -> Vec<u8> {
        let mut row = parent.to_le_bytes().to_vec();
        row.extend_from_slice(&[0; 4]);
        row.extend_from_slice(&sort.to_le_bytes());
        row.extend_from_slice(&id.to_le_bytes());
        row.extend_from_slice(&(folder as u32).to_le_bytes());
        row.extend(wide(name));
        row
    }
    fn entry(index: u32, track: u32, playlist: u32) -> Vec<u8> {
        [index, track, playlist]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect()
    }
    fn sample() -> Vec<u8> {
        let mut pdb = Pdb::new();
        pdb.table(TABLE_ARTISTS, &[artist(7, "Canción")], &[]);
        pdb.table(TABLE_KEYS, &[key(3, "Am")], &[]);
        pdb.table(TABLE_GENRES, &[named(2, &short("House"))], &[]);
        // 20 rows so that a second row group is needed, with one row deleted.
        let rows: Vec<Vec<u8>> = (1..=20)
            .map(|id| track(id, &format!("Pista ñ {id}"), 7, 3))
            .collect();
        pdb.table(TABLE_TRACKS, &rows, &[4]);
        pdb.table(
            TABLE_PLAYLIST_TREE,
            &[
                node(1, 0, "Fiesta", true, 0),
                node(2, 1, "Temprano", false, 1),
            ],
            &[],
        );
        pdb.table(
            TABLE_PLAYLIST_ENTRIES,
            &[entry(2, 3, 2), entry(1, 18, 2)],
            &[],
        );
        pdb.bytes()
    }

    #[test]
    fn a_track_points_to_its_artwork_by_id() {
        let mut pdb = Pdb::new();
        let mut with = track(1, "Con", 0, 0);
        with[0x1c..0x20].copy_from_slice(&9_u32.to_le_bytes());
        let mut missing = track(2, "Perdida", 0, 0);
        missing[0x1c..0x20].copy_from_slice(&10_u32.to_le_bytes());
        pdb.table(TABLE_TRACKS, &[with, missing, track(3, "Sin", 0, 0)], &[]);
        pdb.table(
            TABLE_ARTWORK,
            &[named(9, &short("/PIONEER/Artwork/00001/a9.jpg"))],
            &[],
        );
        let export = parse_export(&pdb.bytes()).unwrap();
        let art: Vec<&str> = export.tracks.iter().map(|t| t.artwork.as_str()).collect();
        assert_eq!(art, ["/PIONEER/Artwork/00001/a9.jpg", "", ""]);
    }

    #[test]
    fn reads_tracks_playlists_and_names() {
        let export = parse_export(&sample()).unwrap();
        assert_eq!(export.tracks.len(), 19, "deleted rows are ignored");
        assert!(export.tracks.iter().all(|t| t.id != 5));
        let last = export.tracks.iter().find(|t| t.id == 18).unwrap();
        assert_eq!(last.title, "Pista ñ 18");
        assert_eq!(last.artist, "Canción");
        assert_eq!(last.genre, "");
        assert_eq!(last.key.as_deref(), Some("Am"));
        assert_eq!(last.bpm, 124.);
        assert_eq!(last.duration, 215);
        assert_eq!(last.rating, 4);
        assert_eq!(last.path, "/Contents/18.mp3");
        let early = export.playlists.iter().find(|n| n.id == 2).unwrap();
        assert_eq!(early.name, "Temprano");
        assert_eq!(early.parent, 1);
        assert!(!early.folder);
        assert_eq!(early.tracks, [18, 3], "in playlist order");
        assert!(export.playlists.iter().find(|n| n.id == 1).unwrap().folder);
    }

    #[test]
    fn damaged_exports_never_panic() {
        let good = sample();
        assert!(parse_export(&[]).is_err());
        assert!(parse_export(b"not a database").is_err());
        for cut in (0..good.len()).step_by(211) {
            let _ = parse_export(&good[..cut]);
        }
        // Flipped bytes across the header and row data, deterministically.
        let mut state = 0x9E37_79B9_u32;
        for _ in 0..400 {
            let mut bad = good.clone();
            for _ in 0..16 {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                let at = state as usize % bad.len();
                bad[at] = (state >> 24) as u8;
            }
            let _ = parse_export(&bad);
        }
    }

    #[test]
    fn page_chains_that_loop_end() {
        let mut pdb = Pdb::new();
        pdb.table(TABLE_TRACKS, &[track(1, "Uno", 0, 0)], &[]);
        let mut bytes = pdb.bytes();
        // Point the page at itself, with a last page that is never reached.
        bytes[PAGE + 0x0c..PAGE + 0x10].copy_from_slice(&1_u32.to_le_bytes());
        bytes[0x1c + 12..0x1c + 16].copy_from_slice(&99_u32.to_le_bytes());
        let tracks = parse_export(&bytes).unwrap().tracks;
        assert_eq!(tracks.len(), 1, "duplicates from the loop collapse");
    }

    fn section(tag: &[u8; 4], header: u32, body: &[u8]) -> Vec<u8> {
        let mut out = tag.to_vec();
        out.extend_from_slice(&header.to_be_bytes());
        out.extend_from_slice(&(header + body.len() as u32).to_be_bytes());
        out.extend_from_slice(&vec![0; header as usize - 12]);
        out.extend_from_slice(body);
        out
    }
    fn file(sections: &[Vec<u8>]) -> Vec<u8> {
        let mut out = b"PMAI".to_vec();
        out.extend_from_slice(&0x1c_u32.to_be_bytes());
        out.extend_from_slice(&[0; 20]);
        for section in sections {
            out.extend_from_slice(section);
        }
        out
    }
    fn grid(beats: &[(u16, u16, u32)]) -> Vec<u8> {
        let body: Vec<u8> = beats
            .iter()
            .flat_map(|(n, t, ms)| {
                let mut b = n.to_be_bytes().to_vec();
                b.extend_from_slice(&t.to_be_bytes());
                b.extend_from_slice(&ms.to_be_bytes());
                b
            })
            .collect();
        let mut s = section(b"PQTZ", 0x18, &body);
        s[0x14..0x18].copy_from_slice(&(beats.len() as u32).to_be_bytes());
        s
    }
    fn basic_list(kind: u32, cues: &[(u32, u8, u32, u32)]) -> Vec<u8> {
        let mut body = Vec::new();
        for (hot, loop_kind, time, end) in cues {
            let mut e = vec![0_u8; 0x38];
            e[..4].copy_from_slice(b"PCPT");
            e[4..8].copy_from_slice(&0x1c_u32.to_be_bytes());
            e[8..12].copy_from_slice(&0x38_u32.to_be_bytes());
            e[0x0c..0x10].copy_from_slice(&hot.to_be_bytes());
            e[0x1c] = *loop_kind;
            e[0x20..0x24].copy_from_slice(&time.to_be_bytes());
            e[0x24..0x28].copy_from_slice(&end.to_be_bytes());
            body.extend(e);
        }
        let mut s = section(b"PCOB", 0x18, &body);
        s[0x0c..0x10].copy_from_slice(&kind.to_be_bytes());
        s[0x12..0x14].copy_from_slice(&(cues.len() as u16).to_be_bytes());
        s
    }
    fn extended_entry(
        hot: u32,
        kind: u8,
        time: u32,
        end: u32,
        label: &str,
        rgb: [u8; 3],
    ) -> Vec<u8> {
        let comment: Vec<u8> = label
            .encode_utf16()
            .chain([0])
            .flat_map(u16::to_be_bytes)
            .collect();
        let mut e = vec![0_u8; 0x2c];
        e[..4].copy_from_slice(b"PCP2");
        e[4..8].copy_from_slice(&0x10_u32.to_be_bytes());
        e[0x0c..0x10].copy_from_slice(&hot.to_be_bytes());
        e[0x10] = kind;
        e[0x14..0x18].copy_from_slice(&time.to_be_bytes());
        e[0x18..0x1c].copy_from_slice(&end.to_be_bytes());
        e[0x28..0x2c].copy_from_slice(&(comment.len() as u32).to_be_bytes());
        e.extend(comment);
        e.extend([1, rgb[0], rgb[1], rgb[2]]);
        let length = e.len() as u32;
        e[8..12].copy_from_slice(&length.to_be_bytes());
        e
    }

    #[test]
    fn reads_the_grid_and_hot_and_memory_cues() {
        let dat = file(&[
            section(b"PPTH", 0x10, &[0, 65, 0, 0]),
            grid(&[(3, 12400, 250), (4, 12400, 734), (1, 12400, 1218)]),
            basic_list(0, &[(0, 1, 5000, NO_TIME)]),
            basic_list(
                1,
                &[
                    (1, 1, 1218, NO_TIME),
                    (3, 2, 2000, 4000),
                    (2, 1, NO_TIME, 0),
                ],
            ),
        ]);
        let analysis = parse_analysis(&dat, None);
        assert_eq!(analysis.beats.len(), 3);
        assert_eq!(analysis.hot[0].as_ref().unwrap().start, 1.218);
        assert_eq!(analysis.hot[1], None, "an unused position is not a cue");
        let looped = analysis.hot[2].as_ref().unwrap();
        assert_eq!((looped.start, looped.end), (2., Some(4.)));
        assert_eq!(analysis.memory[0].start, 5.);
        let (bpm, offset, bar) = constant_grid(&analysis.beats).unwrap();
        assert_eq!((bpm, offset, bar), (124., 0.25, 1.218));
    }

    #[test]
    fn the_extended_list_adds_colors_and_comments() {
        let dat = file(&[basic_list(1, &[(1, 1, 1000, NO_TIME)])]);
        let mut body = extended_entry(1, 1, 1000, NO_TIME, "Drop ñ", [10, 20, 30]);
        body.extend(extended_entry(8, 2, 3000, 5000, "", [0, 0, 0]));
        let mut list = section(b"PCO2", 0x14, &body);
        list[0x10..0x12].copy_from_slice(&2_u16.to_be_bytes());
        let ext = file(&[list]);
        let analysis = parse_analysis(&dat, Some(&ext));
        let first = analysis.hot[0].as_ref().unwrap();
        assert_eq!(first.label.as_deref(), Some("Drop ñ"));
        assert_eq!(first.color, Some([10, 20, 30]));
        let last = analysis.hot[7].as_ref().unwrap();
        assert_eq!(last.end, Some(5.));
        assert_eq!(last.label, None);
    }

    #[test]
    fn damaged_analysis_files_never_panic() {
        let mut body = extended_entry(1, 1, 1000, NO_TIME, "Cue", [1, 2, 3]);
        body.extend(extended_entry(9, 2, 1000, 900, "", [0; 3]));
        let mut list = section(b"PCO2", 0x14, &body);
        list[0x10..0x12].copy_from_slice(&2_u16.to_be_bytes());
        let good = file(&[grid(&[(1, 12000, 0)]), basic_list(1, &[(1, 1, 5, 0)]), list]);
        assert_eq!(parse_analysis(&[], None), Analysis::default());
        for cut in 0..good.len() {
            let _ = parse_analysis(&good[..cut], Some(&good[..cut]));
        }
        let mut state = 0x1234_5678_u32;
        for _ in 0..2000 {
            let mut bad = good.clone();
            for _ in 0..4 {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                let at = state as usize % bad.len();
                bad[at] = (state >> 24) as u8;
            }
            let _ = parse_analysis(&bad, Some(&bad));
        }
    }

    #[test]
    fn keys_read_in_both_notations() {
        let key = |tonic, minor| Some(MusicalKey { tonic, minor });
        assert_eq!(parse_key("Am"), key(9, true));
        assert_eq!(parse_key("C"), key(0, false));
        assert_eq!(parse_key("F#m"), key(6, true));
        assert_eq!(parse_key("Bbm"), key(10, true));
        assert_eq!(parse_key("Eb"), key(3, false));
        assert_eq!(parse_key("Dbmin"), key(1, true));
        assert_eq!(parse_key("8A"), key(9, true));
        assert_eq!(parse_key("8B"), key(0, false));
        assert_eq!(parse_key("9B"), key(7, false));
        assert_eq!(parse_key("12A"), key(1, true));
        assert_eq!(parse_key("1A"), key(8, true));
        assert_eq!(parse_key("1d"), key(0, false));
        assert_eq!(parse_key("1m"), key(9, true));
        assert_eq!(parse_key("2d"), key(7, false));
        assert_eq!(parse_key("6m"), key(8, true));
        assert_eq!(parse_key("12m"), key(2, true));
        // The same key in all three notations.
        assert_eq!(parse_key("Am"), parse_key("8A"));
        assert_eq!(parse_key("8A"), parse_key("1m"));
        assert_eq!(parse_key(""), None);
        assert_eq!(parse_key("13A"), None);
        assert_eq!(parse_key("nada"), None);
    }

    /// `CURADO_TEST_DEVICE=/Volumes/USB cargo test -p curado real_device -- --ignored --nocapture`
    #[test]
    #[ignore = "needs a real rekordbox USB"]
    fn real_device() {
        let root = std::path::PathBuf::from(std::env::var("CURADO_TEST_DEVICE").unwrap());
        let data = std::fs::read(root.join("PIONEER/rekordbox/export.pdb")).unwrap();
        let started = std::time::Instant::now();
        let export = parse_export(&data).unwrap();
        let elapsed = started.elapsed();
        let folders = export.playlists.iter().filter(|p| p.folder).count();
        let entries: usize = export.playlists.iter().map(|p| p.tracks.len()).sum();
        println!(
            "{} tracks, {} playlists, {folders} folders, {entries} entries, parsed in {elapsed:?}",
            export.tracks.len(),
            export.playlists.len() - folders
        );
        let known: std::collections::HashSet<u32> = export.tracks.iter().map(|t| t.id).collect();
        let dangling = export
            .playlists
            .iter()
            .flat_map(|p| &p.tracks)
            .filter(|t| !known.contains(t))
            .count();
        let exists = |p: &str| root.join(p.trim_start_matches('/')).is_file();
        let audio = export.tracks.iter().filter(|t| exists(&t.path)).count();
        let analyzed = export
            .tracks
            .iter()
            .filter(|t| exists(&t.analysis_path))
            .count();
        let with_key = export.tracks.iter().filter(|t| t.key.is_some()).count();
        let readable_key = export
            .tracks
            .iter()
            .filter(|t| t.key.as_deref().and_then(parse_key).is_some())
            .count();
        let unread: std::collections::BTreeSet<&str> = export
            .tracks
            .iter()
            .filter_map(|t| t.key.as_deref())
            .filter(|k| parse_key(k).is_none())
            .collect();
        println!("key names not understood: {unread:?}");
        let titled = export.tracks.iter().filter(|t| !t.title.is_empty()).count();
        let artist = export
            .tracks
            .iter()
            .filter(|t| !t.artist.is_empty())
            .count();
        println!(
            "dangling playlist entries {dangling}; audio found {audio}; analysis found {analyzed}; title {titled}; artist {artist}; key {with_key}, parsed {readable_key}"
        );
        let (mut grids, mut hot, mut memory, mut loops, mut labels, mut colors) =
            (0, 0, 0, 0, 0, 0);
        let (mut on_grid, mut on_bar, mut compared) = (0, 0, 0);
        let mut tempo_mismatch = 0;
        for track in &export.tracks {
            let path = root.join(track.analysis_path.trim_start_matches('/'));
            let Ok(dat) = std::fs::read(&path) else {
                continue;
            };
            let ext = std::fs::read(path.with_extension("EXT")).ok();
            let analysis = parse_analysis(&dat, ext.as_deref());
            if let Some((bpm, _, _)) = constant_grid(&analysis.beats) {
                grids += 1;
                if (bpm - track.bpm).abs() > 0.5 {
                    tempo_mismatch += 1;
                }
            }
            let cues = analysis.hot.iter().flatten().chain(&analysis.memory);
            for cue in cues {
                loops += cue.end.is_some() as usize;
                labels += cue.label.is_some() as usize;
                colors += cue.color.is_some() as usize;
                let ms = (cue.start * 1000.) as i64;
                if let Some(beat) = analysis
                    .beats
                    .iter()
                    .min_by_key(|b| (b.2 as i64 - ms).abs())
                {
                    compared += 1;
                    if (beat.2 as i64 - ms).abs() <= 25 {
                        on_grid += 1;
                        on_bar += (beat.0 == 1) as usize;
                    }
                }
            }
            hot += analysis.hot.iter().flatten().count();
            memory += analysis.memory.len();
        }
        println!(
            "grids {grids} (tempo differs from the track row: {tempo_mismatch}); hot cues {hot}; memory cues {memory}; loops {loops}; labels {labels}; colors {colors}"
        );
        println!(
            "cues compared {compared}; on a grid line {on_grid}; on rekordbox's beat 1 {on_bar}"
        );
    }

    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
        fn below(&mut self, limit: usize) -> usize {
            (self.next() % limit.max(1) as u64) as usize
        }
    }
    /// `CURADO_TEST_DEVICE=/Volumes/X cargo test --release -p curado real_parser_fuzz -- --ignored
    /// --nocapture`. Damages copies in memory; the drive is only read.
    #[test]
    #[ignore = "needs a real rekordbox USB"]
    fn real_parser_fuzz() {
        let root = std::path::PathBuf::from(std::env::var("CURADO_TEST_DEVICE").unwrap());
        let export = std::fs::read(root.join("PIONEER/rekordbox/export.pdb")).unwrap();
        let parsed = parse_export(&export).unwrap();
        let mut rng = Rng(0xFACE_FEED_1234_5671);
        let started = std::time::Instant::now();
        let (mut accepted, mut refused) = (0, 0);
        for round in 0..300 {
            let mut data = export.clone();
            for _ in 0..1 + rng.below(64) {
                // Page headers and row indexes are what the reader trusts most.
                let at = if round % 2 == 0 {
                    rng.below(data.len())
                } else {
                    rng.below(data.len() / 4096) * 4096 + rng.below(64)
                };
                data[at] = rng.next() as u8;
            }
            if round % 5 == 0 {
                data.truncate(rng.below(data.len()).max(1));
            }
            match parse_export(&data) {
                Ok(_) => accepted += 1,
                Err(_) => refused += 1,
            }
        }
        println!(
            "export.pdb: 300 mutations in {:.1} s: {accepted} accepted, {refused} refused",
            started.elapsed().as_secs_f64()
        );
        let mut seeds = Vec::new();
        for track in parsed.tracks.iter().take(40) {
            let path = root.join(track.analysis_path.trim_start_matches('/'));
            if let Ok(dat) = std::fs::read(&path) {
                seeds.push((dat, std::fs::read(path.with_extension("EXT")).ok()));
            }
        }
        assert!(!seeds.is_empty());
        let started = std::time::Instant::now();
        for round in 0..20_000 {
            let (dat, ext) = &seeds[round % seeds.len()];
            let (mut dat, mut ext) = (dat.clone(), ext.clone());
            for _ in 0..1 + rng.below(16) {
                let at = rng.below(dat.len());
                dat[at] = rng.next() as u8;
            }
            if let Some(ext) = &mut ext {
                for _ in 0..rng.below(16) {
                    let at = rng.below(ext.len());
                    ext[at] = rng.next() as u8;
                }
            }
            if round % 5 == 0 {
                dat.truncate(rng.below(dat.len()).max(1));
            }
            let analysis = parse_analysis(&dat, ext.as_deref());
            for cue in analysis.hot.iter().flatten().chain(&analysis.memory) {
                assert!(cue.start.is_finite());
            }
            let _ = constant_grid(&analysis.beats);
        }
        println!(
            "analysis files: 20000 mutations of {} seeds in {:.1} s, no crash",
            seeds.len(),
            started.elapsed().as_secs_f64()
        );
    }

    #[test]
    fn only_a_real_change_of_tempo_makes_a_variable_grid() {
        let steady: Vec<(u16, u16, u32)> = (0..8)
            .map(|i| (1, 12400 + (i % 2), i as u32 * 484))
            .collect();
        assert_eq!(variable_beats(&steady), None);
        let moving: Vec<(u16, u16, u32)> = (0..8)
            .map(|i| (1, 12000 + i * 20, i as u32 * 500))
            .collect();
        assert_eq!(variable_beats(&moving).unwrap().len(), 8);
        assert_eq!(variable_beats(&[]), None);
    }
}
