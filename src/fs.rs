use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

use crate::{error::fs_err, gsl, Result};

/// Removes the given `path` and all of its contents (if it is a directory).
///
/// Does nothing and returns `Ok(())` if `path` does not exist.
pub fn rm_rf(path: impl AsRef<Path>) -> Result<()> {
    _rm_rf(path.as_ref())
}
fn _rm_rf(path: &Path) -> Result<()> {
    let _guard = gsl::read();
    if !path.exists() {
        return Ok(());
    }
    with_path(path, if path.is_file() { std::fs::remove_file(path) } else { remove_dir_all(path) })
}

/// Reads the file at `path` into a [`String`].
pub fn read_file(path: impl AsRef<Path>) -> Result<String> {
    _read_file(path.as_ref())
}
fn _read_file(path: &Path) -> Result<String> {
    let _guard = gsl::read();
    with_path(path, std::fs::read_to_string(path))
}

/// Writes the `contents` into the file at `path`, creating the file (and the
/// path to it) if it didn't exist already.
pub fn write_file(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> Result<()> {
    _write_file(path.as_ref(), contents.as_ref())
}
fn _write_file(path: &Path, contents: &[u8]) -> Result<()> {
    if let Some(p) = path.parent() {
        mkdir_p(p)?;
    }
    let _guard = gsl::read();
    with_path(path, std::fs::write(path, contents))
}

/// Creates the `path` directory and all of its parents.
///
/// Does nothing and returns `Ok(())` if `path` already exists.
pub fn mkdir_p(path: impl AsRef<Path>) -> Result<()> {
    _mkdir_p(path.as_ref())
}
fn _mkdir_p(path: &Path) -> Result<()> {
    let _guard = gsl::read();
    with_path(path, std::fs::create_dir_all(path))
}

/// Copies `src` into `dst`.
///
/// `src` must be a file, but `dst` need not be. If `dst` is an existing
/// directory, `src` will be copied into a file in the `dst` directory whose
/// name is same as that of `src`.
///
/// Otherwise, `dst` is a file or does not exist, and `src` will be copied into
/// it.
pub fn cp(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<()> {
    _cp(src.as_ref(), dst.as_ref())
}
fn _cp(src: &Path, dst: &Path) -> Result<()> {
    let _guard = gsl::read();
    let mut _tmp;
    let mut dst = dst;
    if dst.is_dir() {
        if let Some(file_name) = src.file_name() {
            _tmp = dst.join(file_name);
            dst = &_tmp;
        }
    }
    with_path(src, std::fs::copy(src, dst)).map(|_size| ())
}

/// Returns a sorted list of paths directly contained in the directory at `path`
/// that were able to be accessed without error.
pub fn read_dir(path: impl AsRef<Path>) -> Result<Vec<PathBuf>> {
    _read_dir(path.as_ref())
}
fn _read_dir(path: &Path) -> Result<Vec<PathBuf>> {
    let _guard = gsl::read();
    with_path(path, read_dir_aux(path))
}

/// Returns the current working directory.
pub fn cwd() -> Result<PathBuf> {
    let _guard = gsl::read();
    with_path(&Path::new("."), std::env::current_dir())
}

/// Creates an empty, world-readable, temporary directory.
///
/// Returns a [`TempDir`] value that provides the path of this temporary
/// directory. When dropped, the temporary directory and all of its contents
/// will be removed.
pub fn mktemp_d() -> Result<TempDir> {
    let _guard = gsl::read();
    let base = std::env::temp_dir();
    mkdir_p(&base)?;

    static CNT: AtomicUsize = AtomicUsize::new(0);

    let mut n_try = 0u32;
    loop {
        let cnt = CNT.fetch_add(1, Ordering::Relaxed);
        let path = base.join(format!("xshell-tmp-dir-{}", cnt));
        match std::fs::create_dir(&path) {
            Ok(()) => return Ok(TempDir { path }),
            Err(io_err) if n_try == 1024 => return Err(fs_err(path, io_err)),
            Err(_) => n_try += 1,
        }
    }
}

fn with_path<T>(path: &Path, res: Result<T, std::io::Error>) -> Result<T> {
    res.map_err(|io_err| fs_err(path.to_path_buf(), io_err))
}

#[cfg(not(windows))]
fn remove_dir_all(path: &Path) -> std::io::Result<()> {
    std::fs::remove_dir_all(path)
}

#[cfg(windows)]
fn remove_dir_all(path: &Path) -> std::io::Result<()> {
    for _ in 0..99 {
        if std::fs::remove_dir_all(path).is_ok() {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(10))
    }
    std::fs::remove_dir_all(path)
}

fn read_dir_aux(path: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut res = Vec::new();
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        res.push(entry.path())
    }
    res.sort();
    Ok(res)
}

/// A temporary directory.
#[derive(Debug)]
pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    /// Returns the path of this temporary directory.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = rm_rf(&self.path);
    }
}
