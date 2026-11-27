//! Atomic publication of the files a process wrote to `partial` paths.
//!
//! A partial output is never exposed: it is validated (regular file, minimum
//! size), fsynced and only then placed at `target`, exclusively unless the
//! output says `replace`. A failed placement rolls back the exclusive ones.
use crate::sys;
use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

pub struct Output {
    pub partial: PathBuf,
    pub target: PathBuf,
    pub min_bytes: u64,
    pub replace: bool,
}

#[derive(Debug)]
pub enum PublishError {
    /// The partial is missing, not a regular file or shorter than `min_bytes`.
    Invalid(PathBuf),
    /// fsync of the partial failed.
    Sync(PathBuf, io::Error),
    /// Placing at the target failed; exclusive placements were rolled back.
    Place(PathBuf, io::Error),
}

enum Placed {
    /// Hard link (exclusive): the partial still exists.
    Linked,
    /// Renamed: the partial is gone.
    Moved,
}

/// Publish `partial` at `target` without ever exposing a half-written file.
fn place(partial: &Path, target: &Path, replace: bool) -> io::Result<Placed> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    if replace {
        return match fs::rename(partial, target) {
            Ok(()) => Ok(Placed::Moved),
            Err(error) if error.raw_os_error() == Some(sys::EXDEV) => {
                let staged = stage_copy(partial, target)?;
                fs::rename(&staged, target).inspect_err(|_| {
                    let _ = fs::remove_file(&staged);
                })?;
                Ok(Placed::Linked)
            }
            Err(error) => Err(error),
        };
    }
    match fs::hard_link(partial, target) {
        Ok(()) => Ok(Placed::Linked),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Err(error),
        Err(error) if error.raw_os_error() == Some(sys::EXDEV) => {
            let staged = stage_copy(partial, target)?;
            let result = exclusive_move(&staged, target);
            let _ = fs::remove_file(&staged);
            result.map(|_| Placed::Linked)
        }
        // File systems without hard links (exFAT): exclusive check, then atomic rename.
        Err(_) => exclusive_move(partial, target),
    }
}

fn exclusive_move(source: &Path, target: &Path) -> io::Result<Placed> {
    match fs::hard_link(source, target) {
        Ok(()) => Ok(Placed::Linked),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Err(error),
        Err(_) => {
            if fs::symlink_metadata(target).is_ok() {
                return Err(io::Error::from(io::ErrorKind::AlreadyExists));
            }
            fs::rename(source, target)?;
            Ok(Placed::Moved)
        }
    }
}

/// Copy next to the target (same file system), fsynced, under a hidden name.
fn stage_copy(partial: &Path, target: &Path) -> io::Result<PathBuf> {
    let name = target
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let directory = target.parent().unwrap_or(Path::new("/"));
    for attempt in 0..100 {
        let staged = directory.join(format!(
            ".{name}.publishing-{}-{attempt}",
            std::process::id()
        ));
        let mut file = match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&staged)
        {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        };
        let copied = File::open(partial)
            .and_then(|mut source| io::copy(&mut source, &mut file))
            .and_then(|_| file.sync_all());
        if let Err(error) = copied {
            let _ = fs::remove_file(&staged);
            return Err(error);
        }
        return Ok(staged);
    }
    Err(io::Error::other("no free staging name"))
}

/// Validate, sync and place every output; returns the sorted published targets.
pub fn publish(outputs: &[Output]) -> Result<Vec<String>, PublishError> {
    for output in outputs {
        let valid = fs::symlink_metadata(&output.partial)
            .is_ok_and(|meta| meta.is_file() && meta.len() >= output.min_bytes);
        if !valid {
            return Err(PublishError::Invalid(output.partial.clone()));
        }
    }
    for output in outputs {
        if let Err(error) = File::open(&output.partial).and_then(|file| file.sync_all()) {
            return Err(PublishError::Sync(output.partial.clone(), error));
        }
    }
    // Exclusive outputs first (they can be rolled back), replacements last.
    let mut order: Vec<&Output> = outputs.iter().filter(|output| !output.replace).collect();
    order.extend(outputs.iter().filter(|output| output.replace));
    let mut placed: Vec<(&Output, Placed)> = Vec::new();
    for output in order {
        match place(&output.partial, &output.target, output.replace) {
            Ok(how) => placed.push((output, how)),
            Err(error) => {
                for (done, how) in placed.iter().rev() {
                    if done.replace {
                        continue;
                    }
                    match how {
                        Placed::Linked => {
                            let _ = fs::remove_file(&done.target);
                        }
                        Placed::Moved => {
                            let _ = fs::rename(&done.target, &done.partial);
                        }
                    }
                }
                return Err(PublishError::Place(output.target.clone(), error));
            }
        }
    }
    let mut published = Vec::new();
    for (output, how) in &placed {
        if matches!(how, Placed::Linked) {
            let _ = fs::remove_file(&output.partial);
        }
        if let Some(parent) = output.target.parent()
            && let Ok(handle) = File::open(parent)
        {
            let _ = handle.sync_all();
        }
        published.push(output.target.to_string_lossy().into_owned());
    }
    published.sort();
    Ok(published)
}
