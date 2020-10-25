use std::path::{Path, PathBuf};

use crate::{error::fs_err, gsl, Result};

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

pub fn read_file(path: impl AsRef<Path>) -> Result<String> {
    _read_file(path.as_ref())
}
fn _read_file(path: &Path) -> Result<String> {
    let _guard = gsl::read();
    with_path(path, std::fs::read_to_string(path))
}

pub fn write_file(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> Result<()> {
    _write_file(path.as_ref(), contents.as_ref())
}
fn _write_file(path: &Path, contents: &[u8]) -> Result<()> {
    let _guard = gsl::read();
    with_path(path, std::fs::write(path, contents))
}

pub fn mkdir_p(path: impl AsRef<Path>) -> Result<()> {
    _mkdir_p(path.as_ref())
}
fn _mkdir_p(path: &Path) -> Result<()> {
    let _guard = gsl::read();
    with_path(path, std::fs::create_dir_all(path))
}

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

pub fn read_dir(path: impl AsRef<Path>) -> Result<Vec<PathBuf>> {
    _read_dir(path.as_ref())
}
fn _read_dir(path: &Path) -> Result<Vec<PathBuf>> {
    let _guard = gsl::read();
    with_path(path, read_dir_aux(path))
}

pub fn cwd() -> Result<PathBuf> {
    let _guard = gsl::read();
    with_path(&Path::new("."), std::env::current_dir())
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
