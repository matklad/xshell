use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

use crate::{cwd, error::fs_err, gsl, Result};

/// Changes the current directory to `dir`.
///
/// Returns a [`Pushd`] value that, when dropped, will reset the current
/// directory to whatever it was right before the call to `pushd` that produced
/// that `Pushd`.
pub fn pushd(dir: impl AsRef<Path>) -> Result<Pushd> {
    Pushd::new(dir.as_ref())
}

/// The result of calling a successful [`pushd`].
#[must_use]
#[derive(Debug)]
pub struct Pushd {
    _guard: gsl::Guard,
    prev_dir: PathBuf,
    dir: PathBuf,
}

/// Sets the environment variable `key` to have value `val`.
///
/// Returns a [`Pushenv`] value that, when dropped, will reset the the
/// environment variable `key` to whatever value it had right before the call to
/// `pushenv` that produced that `Pushenv`.
pub fn pushenv(key: impl AsRef<OsStr>, val: impl AsRef<OsStr>) -> Pushenv {
    Pushenv::new(key.as_ref(), val.as_ref())
}

/// The result of calling a successful [`pushenv`].
#[must_use]
#[derive(Debug)]
pub struct Pushenv {
    _guard: gsl::Guard,
    key: OsString,
    prev_value: Option<OsString>,
    value: OsString,
}

impl Pushd {
    fn new(dir: &Path) -> Result<Pushd> {
        let guard = gsl::write();
        let prev_dir = cwd()?;
        set_current_dir(&dir)?;
        let dir = cwd()?;
        Ok(Pushd { _guard: guard, prev_dir, dir })
    }
}

impl Drop for Pushd {
    fn drop(&mut self) {
        let dir = cwd().unwrap();
        assert_eq!(
            dir,
            self.dir,
            "current directory was changed concurrently.
expected {}
got      {}",
            self.dir.display(),
            dir.display()
        );
        set_current_dir(&self.prev_dir).unwrap()
    }
}

fn set_current_dir(path: &Path) -> Result<()> {
    std::env::set_current_dir(path).map_err(|err| fs_err(path.to_path_buf(), err))
}

impl Pushenv {
    fn new(key: &OsStr, value: &OsStr) -> Pushenv {
        let guard = gsl::write();
        let prev_value = std::env::var_os(key);
        std::env::set_var(key, value);
        Pushenv { _guard: guard, key: key.to_os_string(), prev_value, value: value.to_os_string() }
    }
}

impl Drop for Pushenv {
    fn drop(&mut self) {
        let value = std::env::var_os(&self.key);
        assert_eq!(
            value.as_ref(),
            Some(&self.value),
            "environmental variable was changed concurrently.
var      {:?}
expected {:?}
got      {:?}",
            self.key,
            self.value,
            value
        );
        match &self.prev_value {
            Some(it) => std::env::set_var(&self.key, &it),
            None => std::env::remove_var(&self.key),
        }
    }
}
