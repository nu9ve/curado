//! What a file is, apart from where it is. A path breaks when a file or a drive moves; size, a
//! fingerprint and the drive's own id let curado find it again and tell two copies apart.
use sha2::{Digest, Sha256};
#[cfg(not(windows))]
use std::process::Command;
use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    sync::Mutex,
};

/// Enough of each end to tell files apart without reading a whole track from a slow drive.
const SAMPLED: u64 = 256 * 1024;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Identity {
    pub size: u64,
    pub fingerprint: String,
    /// The drive's own id and the path inside it. Nothing for the startup disk.
    pub volume: Option<String>,
    pub relative: Option<String>,
}

/// Size and both ends of the file. Any tagging or editing changes it.
pub fn fingerprint(path: &Path) -> Result<(u64, String), String> {
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let size = file.metadata().map_err(|e| e.to_string())?.len();
    let mut hash = Sha256::new();
    hash.update(size.to_le_bytes());
    let mut sample = vec![0; SAMPLED.min(size) as usize];
    file.read_exact(&mut sample).map_err(|e| e.to_string())?;
    hash.update(&sample);
    if size > SAMPLED {
        let tail = SAMPLED.min(size - SAMPLED);
        file.seek(SeekFrom::End(-(tail as i64)))
            .map_err(|e| e.to_string())?;
        let mut sample = vec![0; tail as usize];
        file.read_exact(&mut sample).map_err(|e| e.to_string())?;
        hash.update(&sample);
    }
    Ok((size, hex(&hash.finalize()[..16])))
}
/// The whole file, for what curado writes itself and may have to prove later.
pub fn sha256(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    std::io::copy(&mut file, &mut hash).map_err(|e| e.to_string())?;
    Ok(hex(&hash.finalize()))
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn identify(path: &Path) -> Result<Identity, String> {
    let (size, fingerprint) = fingerprint(path)?;
    let (volume, relative) = match volume_root(path) {
        Some(root) => (
            volume_id(&root),
            path.strip_prefix(&root)
                .ok()
                .map(|inner| inner.to_string_lossy().to_string()),
        ),
        None => (None, None),
    };
    Ok(Identity {
        size,
        fingerprint,
        volume,
        relative,
    })
}
/// The drive a path is on, when it is not the startup disk.
#[cfg(not(windows))]
pub fn volume_root(path: &Path) -> Option<PathBuf> {
    let inner = path.strip_prefix("/Volumes").ok()?;
    Some(Path::new("/Volumes").join(inner.components().next()?))
}
/// On Windows, a drive letter from D to Z, the same ones `devices::mount_points` lists. The
/// root keeps the letter as the path spells it, so `strip_prefix` matches it.
#[cfg(windows)]
pub fn volume_root(path: &Path) -> Option<PathBuf> {
    use std::path::{Component, Prefix};
    let mut parts = path.components();
    let letter = match parts.next()? {
        Component::Prefix(prefix) => match prefix.kind() {
            Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) => letter,
            _ => return None,
        },
        _ => return None,
    };
    if !matches!(parts.next()?, Component::RootDir)
        || !(b'D'..=b'Z').contains(&letter.to_ascii_uppercase())
    {
        return None;
    }
    Some(path.components().take(2).collect())
}
static VOLUME_IDS: Mutex<Option<HashMap<PathBuf, Option<String>>>> = Mutex::new(None);
/// Asked of the system once per mount: it takes a moment and does not change.
pub fn volume_id(root: &Path) -> Option<String> {
    let mut known = VOLUME_IDS.lock().unwrap();
    known
        .get_or_insert_with(HashMap::new)
        .entry(root.to_path_buf())
        .or_insert_with(|| read_volume_id(root))
        .clone()
}
/// Drives come and go: another one can appear under a name already seen.
pub fn forget_volumes() {
    *VOLUME_IDS.lock().unwrap() = None;
}
#[cfg(not(windows))]
fn read_volume_id(root: &Path) -> Option<String> {
    let output = Command::new("/usr/sbin/diskutil")
        .args(["info", "-plist"])
        .arg(root)
        .output()
        .ok()?;
    parse_volume_id(&String::from_utf8_lossy(&output.stdout))
}
#[cfg(not(windows))]
fn parse_volume_id(plist: &str) -> Option<String> {
    let after = plist.split("<key>VolumeUUID</key>").nth(1)?;
    let value = after.split("<string>").nth(1)?.split("</string>").next()?;
    (!value.is_empty()).then(|| value.to_string())
}
/// Free space where a path lives, when the system says.
#[cfg(not(windows))]
pub fn free_bytes(path: &Path) -> Option<u64> {
    let output = Command::new("/bin/df").arg("-k").arg(path).output().ok()?;
    parse_free(&String::from_utf8_lossy(&output.stdout))
}
#[cfg(not(windows))]
fn parse_free(table: &str) -> Option<u64> {
    let kib: u64 = table
        .lines()
        .nth(1)?
        .split_whitespace()
        .nth(3)?
        .parse()
        .ok()?;
    Some(kib * 1024)
}

/// Windows answers both questions through kernel32 directly: there is no `df` or `diskutil`.
#[cfg(windows)]
fn wide(path: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    path.as_os_str().encode_wide().chain(Some(0)).collect()
}
/// The volume serial number, which changes when a drive is formatted, like a macOS VolumeUUID.
#[cfg(windows)]
fn read_volume_id(root: &Path) -> Option<String> {
    use windows_sys::Win32::Storage::FileSystem::GetVolumeInformationW;
    let root = wide(root);
    let mut serial = 0_u32;
    // SAFETY: `root` is NUL-terminated and outlives the call; the only out pointer is `serial`.
    let ok = unsafe {
        GetVolumeInformationW(
            root.as_ptr(),
            std::ptr::null_mut(),
            0,
            &mut serial,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0,
        )
    };
    (ok != 0).then(|| format!("{serial:08X}"))
}
/// Space available to this user, which respects quotas, as `df` reports it on Unix.
#[cfg(windows)]
pub fn free_bytes(path: &Path) -> Option<u64> {
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    let path = wide(path);
    let mut available = 0_u64;
    // SAFETY: `path` is NUL-terminated and outlives the call; null out pointers are allowed.
    let ok = unsafe {
        GetDiskFreeSpaceExW(
            path.as_ptr(),
            &mut available,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    (ok != 0).then_some(available)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn temporary(name: &str, bytes: &[u8]) -> PathBuf {
        let path = std::env::temp_dir().join(format!("curado-id-{}-{name}", std::process::id()));
        std::fs::write(&path, bytes).unwrap();
        path
    }
    #[test]
    fn a_fingerprint_follows_the_content_not_the_name() {
        let body = vec![7_u8; 900_000];
        let (a, b) = (temporary("a", &body), temporary("b", &body));
        assert_eq!(fingerprint(&a).unwrap(), fingerprint(&b).unwrap());
        // A change near either end, as tagging makes, or of size.
        let mut start = body.clone();
        start[10] = 8;
        let mut end = body.clone();
        end[899_000] = 8;
        let mut longer = body.clone();
        longer.push(7);
        for (name, other) in [("c", start), ("d", end), ("e", longer)] {
            let other = temporary(name, &other);
            assert_ne!(
                fingerprint(&a).unwrap(),
                fingerprint(&other).unwrap(),
                "{name}"
            );
            let _ = std::fs::remove_file(other);
        }
        let _ = std::fs::remove_file(a);
        let _ = std::fs::remove_file(b);
    }
    #[test]
    fn small_and_empty_files_have_one_too() {
        for (name, body) in [("s", &b"tiny"[..]), ("z", &b""[..])] {
            let path = temporary(name, body);
            assert_eq!(fingerprint(&path).unwrap().0, body.len() as u64);
            let _ = std::fs::remove_file(path);
        }
        assert!(fingerprint(Path::new("/nowhere/at/all")).is_err());
    }
    #[test]
    #[cfg(not(windows))]
    fn only_outside_drives_have_a_volume() {
        assert_eq!(
            volume_root(Path::new("/Volumes/SETS/music/a.mp3")),
            Some(PathBuf::from("/Volumes/SETS"))
        );
        assert_eq!(volume_root(Path::new("/Users/me/a.mp3")), None);
        assert_eq!(volume_root(Path::new("/Volumes")), None);
    }
    #[test]
    #[cfg(windows)]
    fn only_outside_drives_have_a_volume() {
        let path = Path::new(r"E:\SETS\music\a.mp3");
        let root = volume_root(path).unwrap();
        assert_eq!(root, PathBuf::from(r"E:\"));
        assert_eq!(
            path.strip_prefix(&root).unwrap(),
            Path::new(r"SETS\music\a.mp3")
        );
        // The letter keeps its spelling, or `strip_prefix` would not match.
        let lower = Path::new(r"e:\a.mp3");
        assert!(lower.strip_prefix(volume_root(lower).unwrap()).is_ok());
        assert_eq!(volume_root(Path::new(r"C:\Users\me\a.mp3")), None);
        assert_eq!(volume_root(Path::new(r"\\server\share\a.mp3")), None);
        assert_eq!(volume_root(Path::new(r"E:relative.mp3")), None);
        assert_eq!(volume_root(Path::new("relative.mp3")), None);
    }
    #[test]
    #[cfg(windows)]
    fn windows_answers_come_from_the_system() {
        let system = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
        let root = PathBuf::from(format!(r"{system}\"));
        let id = read_volume_id(&root).unwrap();
        assert_eq!(id.len(), 8, "{id}");
        assert!(free_bytes(&std::env::temp_dir()).is_some_and(|free| free > 0));
        assert_eq!(read_volume_id(Path::new(r"Q:\nowhere\")), None);
    }
    #[test]
    #[cfg(not(windows))]
    fn the_system_answers_are_read_carefully() {
        let plist = "<key>VolumeName</key><string>X</string>\n<key>VolumeUUID</key>\n\t<string>AB-CD</string>";
        assert_eq!(parse_volume_id(plist).as_deref(), Some("AB-CD"));
        assert_eq!(parse_volume_id("<key>Other</key>"), None);
        assert_eq!(parse_volume_id(""), None);
        let table = "Filesystem 1024-blocks Used Available Capacity\n/dev/disk1 100 40 60 40% /";
        assert_eq!(parse_free(table), Some(60 * 1024));
        assert_eq!(parse_free("nonsense"), None);
    }
}
