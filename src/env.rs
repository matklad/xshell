use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

use crate::{cwd, error::fs_err, gsl, Result};

pub fn pushd(dir: impl AsRef<Path>) -> Result<Pushd> {
    Pushd::new(dir.as_ref())
}

#[must_use]
pub struct Pushd {
    _guard: gsl::Guard,
    prev_dir: PathBuf,
    dir: PathBuf,
}

pub fn pushenv(k: impl AsRef<OsStr>, v: impl AsRef<OsStr>) -> Pushenv {
    Pushenv::new(k.as_ref(), v.as_ref())
}

#[must_use]
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
