//! xshell is a swiss-army knife for writing cross-platform "bash" scripts in
//! Rust.
//!
//! It doesn't use the shell directly, but rather re-implements parts of
//! scripting environment in Rust. The intended use-case is various bits of glue
//! code, which could be written in bash or python. The original motivation is
//! [`xtask`](https://github.com/matklad/cargo-xtask) development.
//!
//! Here's a quick example:
//!
//! ```no_run
//! use xshell::{Shell, cmd};
//!
//! let sh = Shell::new()?;
//! let branch = "main";
//! let commit_hash = cmd!(sh, "git rev-parse {branch}").read()?;
//! # Ok::<(), xshell::Error>(())
//! ```
//!
//! **Goals:**
//!
//! * Ergonomics and DWIM ("do what I mean"): `cmd!` macro supports
//!   interpolation, writing to a file automatically creates parent directories,
//!   etc.
//! * Reliability: no [shell injection] by construction, good error messages
//!   with file  paths, non-zero exit status is an error, independence of the
//!   host environment, etc.
//! * Frugality: fast compile times, few dependencies, low-tech API.
//!
//! # Guide
//!
//! For a short API overview, let's implement a script to clone a github
//! repository and publish it as a crates.io crate. The script will do the
//! following:
//!
//! 1. Clone the repository.
//! 2. `cd` into the repository's directory.
//! 3. Run the tests.
//! 4. Create a git tag using a version from `Cargo.toml`.
//! 5. Publish the crate with an optional `--dry-run`.
//!
//! Start with the following skeleton:
//!
//! ```no_run
//! use xshell::{cmd, Shell};
//!
//! fn main() -> anyhow::Result<()> {
//!     let sh = Shell::new()?;
//!
//!     Ok(())
//! }
//! ```
//!
//! Only two imports are needed -- the [`Shell`] struct the and [`cmd!`] macro.
//! By convention, an instance of a [`Shell`] is stored in a variable named
//! `sh`. All the API is available as methods, so a short name helps here. For
//! "scripts", the [`anyhow`](https://docs.rs/anyhow) crate is a great choice
//! for an error-handling library.
//!
//! Next, clone the repository:
//!
//! ```no_run
//! # use xshell::{Shell, cmd}; let sh = Shell::new().unwrap();
//! cmd!(sh, "git clone https://github.com/matklad/xshell.git").run()?;
//! # Ok::<(), xshell::Error>(())
//! ```
//!
//! The [`cmd!`] macro provides a convenient syntax for creating a command --
//! the [`Cmd`] struct. The [`Cmd::run`] method runs the command as if you
//! typed it into the shell. The whole program outputs:
//!
//! ```console
//! $ git clone https://github.com/matklad/xshell.git
//! Cloning into 'xshell'...
//! remote: Enumerating objects: 676, done.
//! remote: Counting objects: 100% (220/220), done.
//! remote: Compressing objects: 100% (123/123), done.
//! remote: Total 676 (delta 106), reused 162 (delta 76), pack-reused 456
//! Receiving objects: 100% (676/676), 136.80 KiB | 222.00 KiB/s, done.
//! Resolving deltas: 100% (327/327), done.
//! ```
//!
//! Note that the command itself is echoed to stderr (the `$ git ...` bit in the
//! output). You can use [`Cmd::quiet`] to override this behavior:
//!
//! ```no_run
//! # use xshell::{Shell, cmd}; let sh = Shell::new().unwrap();
//! cmd!(sh, "git clone https://github.com/matklad/xshell.git")
//!     .quiet()
//!     .run()?;
//! # Ok::<(), xshell::Error>(())
//! ```
//!
//! To make the code more general, let's use command interpolation to extract
//! the username and the repository:
//!
//! ```no_run
//! # use xshell::{Shell, cmd}; let sh = Shell::new().unwrap();
//! let user = "matklad";
//! let repo = "xshell";
//! cmd!(sh, "git clone https://github.com/{user}/{repo}.git").run()?;
//! # Ok::<(), xshell::Error>(())
//! ```
//!
//! Note that the `cmd!` macro parses the command string at compile time, so you
//! don't have to worry about escaping the arguments. For example, the following
//! command "touches" a single file whose name is `contains a space`:
//!
//! ```no_run
//! # use xshell::{Shell, cmd}; let sh = Shell::new().unwrap();
//! let file = "contains a space";
//! cmd!(sh, "touch {file}").run()?;
//! # Ok::<(), xshell::Error>(())
//! ```
//!
//! Next, `cd` into the folder you have just cloned:
//!
//! ```no_run
//! # use xshell::{Shell, cmd}; let sh = Shell::new().unwrap();
//! # let repo = "xshell";
//! sh.change_dir(repo);
//! ```
//!
//! Each instance of [`Shell`] has a current directory, which is independent of
//! the process-wide [`std::env::current_dir`]. The same applies to the
//! environment.
//!
//! Next, run the tests:
//!
//! ```no_run
//! # use xshell::{Shell, cmd}; let sh = Shell::new().unwrap();
//! let test_args = ["-Zunstable-options", "--report-time"];
//! cmd!(sh, "cargo test -- {test_args...}").run()?;
//! # Ok::<(), xshell::Error>(())
//! ```
//!
//! Note how the so-called splat syntax (`...`) is used to interpolate an
//! iterable of arguments.
//!
//! Next, read the Cargo.toml so that we can fetch crate' declared version:
//!
//! ```no_run
//! # use xshell::{Shell, cmd}; let sh = Shell::new().unwrap();
//! let manifest = sh.read_file("Cargo.toml")?;
//! # Ok::<(), xshell::Error>(())
//! ```
//!
//! [`Shell::read_file`] works like [`std::fs::read_to_string`], but paths are
//! relative to the current directory of the [`Shell`]. Unlike [`std::fs`],
//! error messages are much more useful. For example, if there isn't a
//! `Cargo.toml` in the repository, the error message is:
//!
//! ```text
//! Error: failed to read file `xshell/Cargo.toml`: no such file or directory (os error 2)
//! ```
//!
//! `xshell` doesn't implement string processing utils like `grep`, `sed` or
//! `awk` -- there's no need to, built-in language features work fine, and it's
//! always possible to pull extra functionality from crates.io.
//!
//! To extract the `version` field from Cargo.toml, [`str::split_once`] is
//! enough:
//!
//! ```no_run
//! # use xshell::{Shell, cmd}; let sh = Shell::new().unwrap();
//! let manifest = sh.read_file("Cargo.toml")?;
//! let version = manifest
//!     .split_once("version = \"")
//!     .and_then(|it| it.1.split_once('\"'))
//!     .map(|it| it.0)
//!     .ok_or_else(|| anyhow::format_err!("can't find version field in the manifest"))?;
//!
//! cmd!(sh, "git tag {version}").run()?;
//! # Ok::<(), anyhow::Error>(())
//! ```
//!
//! The splat (`...`) syntax works with any iterable, and in Rust options are
//! iterable. This means that `...` can be used to implement optional arguments.
//! For example, here's how to pass `--dry-run` when *not* running in CI:
//!
//! ```no_run
//! # use xshell::{Shell, cmd}; let sh = Shell::new().unwrap();
//! let dry_run = if sh.var("CI").is_ok() { None } else { Some("--dry-run") };
//! cmd!(sh, "cargo publish {dry_run...}").run()?;
//! # Ok::<(), xshell::Error>(())
//! ```
//!
//! Putting everything altogether, here's the whole script:
//!
//! ```no_run
//! use xshell::{cmd, Shell};
//!
//! fn main() -> anyhow::Result<()> {
//!     let sh = Shell::new()?;
//!
//!     let user = "matklad";
//!     let repo = "xshell";
//!     cmd!(sh, "git clone https://github.com/{user}/{repo}.git").run()?;
//!     sh.change_dir(repo);
//!
//!     let test_args = ["-Zunstable-options", "--report-time"];
//!     cmd!(sh, "cargo test -- {test_args...}").run()?;
//!
//!     let manifest = sh.read_file("Cargo.toml")?;
//!     let version = manifest
//!         .split_once("version = \"")
//!         .and_then(|it| it.1.split_once('\"'))
//!         .map(|it| it.0)
//!         .ok_or_else(|| anyhow::format_err!("can't find version field in the manifest"))?;
//!
//!     cmd!(sh, "git tag {version}").run()?;
//!
//!     let dry_run = if sh.var("CI").is_ok() { None } else { Some("--dry-run") };
//!     cmd!(sh, "cargo publish {dry_run...}").run()?;
//!
//!     Ok(())
//! }
//! ```
//!
//! `xshell` itself uses a similar script to automatically publish oneself to
//! crates.io when the version in Cargo.toml changes:
//!
//! <https://github.com/matklad/xshell/blob/master/examples/ci.rs>
//!
//! # Maintenance
//!
//! Minimum Supported Rust Version: 1.59.0. MSRV bump is not considered semver
//! breaking. MSRV is updated conservatively.
//!
//! The crate isn't comprehensive yet, but this is a goal. You are hereby
//! encouraged to submit PRs with missing functionality!
//!
//! # Related Crates
//!
//! [`duct`] is a crate for heavy-duty process herding, with support for
//! pipelines.
//!
//! Most of what this crate provides can be open-coded using
//! [`std::process::Command`] and [`std::fs`]. If you only need to spawn a
//! single process, using `std` is probably better (but don't forget to check
//! the exit status!).
//!
//! [`duct`]: https://github.com/oconnor663/duct.rs
//! [shell injection]:
//!     https://en.wikipedia.org/wiki/Code_injection#Shell_injection
//!
//! The [`dax`](https://github.com/dsherret/dax) library for Deno shares the overall philosophy with
//! `xshell`, but is much more thorough and complete. If you don't need Rust, use `dax`.
//!
//! # Implementation Notes
//!
//! The design is heavily inspired by the Julia language:
//!
//! * [Shelling Out
//!   Sucks](https://julialang.org/blog/2012/03/shelling-out-sucks/)
//! * [Put This In Your
//!   Pipe](https://julialang.org/blog/2013/04/put-this-in-your-pipe/)
//! * [Running External
//!   Programs](https://docs.julialang.org/en/v1/manual/running-external-programs/)
//! * [Filesystem](https://docs.julialang.org/en/v1/base/file/)
//!
//! Smaller influences are the [`duct`] crate and Ruby's
//! [`FileUtils`](https://ruby-doc.org/stdlib-2.4.1/libdoc/fileutils/rdoc/FileUtils.html)
//! module.
//!
//! The `cmd!` macro uses a simple proc-macro internally. It doesn't depend on
//! helper libraries, so the fixed-cost impact on compile times is moderate.
//! Compiling a trivial program with `cmd!("date +%Y-%m-%d")` takes one second.
//! Equivalent program using only `std::process::Command` compiles in 0.25
//! seconds.
//!
//! To make IDEs infer correct types without expanding proc-macro, it is wrapped
//! into a declarative macro which supplies type hints.

#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![deny(rust_2018_idioms)]

mod error;

use std::{
    cell::RefCell,
    collections::HashMap,
    env::{self, current_dir, VarError},
    ffi::{OsStr, OsString},
    fmt, fs,
    io::{self, ErrorKind, Write},
    mem,
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Output, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
};

pub use crate::error::{Error, Result};
#[doc(hidden)]
pub use xshell_macros::__cmd;

/// Constructs a [`Cmd`] from the given string.
///
/// # Examples
///
/// Basic:
///
/// ```no_run
/// # use xshell::{cmd, Shell};
/// let sh = Shell::new()?;
/// cmd!(sh, "echo hello world").run()?;
/// # Ok::<(), xshell::Error>(())
/// ```
///
/// Interpolation:
///
/// ```
/// # use xshell::{cmd, Shell}; let sh = Shell::new()?;
/// let greeting = "hello world";
/// let c = cmd!(sh, "echo {greeting}");
/// assert_eq!(c.to_string(), r#"echo "hello world""#);
///
/// let c = cmd!(sh, "echo '{greeting}'");
/// assert_eq!(c.to_string(), r#"echo {greeting}"#);
///
/// let c = cmd!(sh, "echo {greeting}!");
/// assert_eq!(c.to_string(), r#"echo "hello world!""#);
///
/// let c = cmd!(sh, "echo 'spaces '{greeting}' around'");
/// assert_eq!(c.to_string(), r#"echo "spaces hello world around""#);
///
/// # Ok::<(), xshell::Error>(())
/// ```
///
/// Splat interpolation:
///
/// ```
/// # use xshell::{cmd, Shell}; let sh = Shell::new()?;
/// let args = ["hello", "world"];
/// let c = cmd!(sh, "echo {args...}");
/// assert_eq!(c.to_string(), r#"echo hello world"#);
///
/// let arg1: Option<&str> = Some("hello");
/// let arg2: Option<&str> = None;
/// let c = cmd!(sh, "echo {arg1...} {arg2...}");
/// assert_eq!(c.to_string(), r#"echo hello"#);
/// # Ok::<(), xshell::Error>(())
/// ```
#[macro_export]
macro_rules! cmd {
    ($sh:expr, $cmd:literal) => {{
        #[cfg(trick_rust_analyzer_into_highlighting_interpolated_bits)]
        format_args!($cmd);
        let f = |prog| $sh.cmd(prog);
        let cmd: $crate::Cmd = $crate::__cmd!(f $cmd);
        cmd
    }};
}

/// A `Shell` is the main API entry point.
///
/// Almost all of the crate's functionality is available as methods of the
/// `Shell` object.
///
/// `Shell` is a stateful object. It maintains a logical working directory and
/// an environment map. They are independent from process's
/// [`std::env::current_dir`] and [`std::env::var`], and only affect paths and
/// commands passed to the [`Shell`].
///
///
/// By convention, variable holding the shell is named `sh`.
///
/// # Example
///
/// ```no_run
/// use xshell::{cmd, Shell};
///
/// let sh = Shell::new()?;
/// let _d = sh.push_dir("./target");
/// let cwd = sh.current_dir();
/// cmd!(sh, "echo current dir is {cwd}").run()?;
///
/// let process_cwd = std::env::current_dir().unwrap();
/// assert_eq!(cwd, process_cwd.join("./target"));
/// # Ok::<(), xshell::Error>(())
/// ```
#[derive(Debug)]
pub struct Shell {
    cwd: RefCell<PathBuf>,
    env: RefCell<HashMap<OsString, OsString>>,
}

impl std::panic::UnwindSafe for Shell {}
impl std::panic::RefUnwindSafe for Shell {}

impl Shell {
    /// Creates a new [`Shell`].
    ///
    /// Fails if [`std::env::current_dir`] returns an error.
    pub fn new() -> Result<Shell> {
        let cwd = current_dir().map_err(|err| Error::new_current_dir(err, None))?;
        let cwd = RefCell::new(cwd);
        let env = RefCell::new(HashMap::new());
        Ok(Shell { cwd, env })
    }

    // region:env
    /// Returns the working directory for this [`Shell`].
    ///
    /// All relative paths are interpreted relative to this directory, rather
    /// than [`std::env::current_dir`].
    #[doc(alias = "pwd")]
    pub fn current_dir(&self) -> PathBuf {
        self.cwd.borrow().clone()
    }

    /// Changes the working directory for this [`Shell`].
    ///
    /// Note that this doesn't affect [`std::env::current_dir`].
    #[doc(alias = "pwd")]
    pub fn change_dir<P: AsRef<Path>>(&self, dir: P) {
        self._change_dir(dir.as_ref())
    }
    fn _change_dir(&self, dir: &Path) {
        let dir = self.path(dir);
        *self.cwd.borrow_mut() = dir;
    }

    /// Temporary changes the working directory of this [`Shell`].
    ///
    /// Returns a RAII guard which reverts the working directory to the old
    /// value when dropped.
    ///
    /// Note that this doesn't affect [`std::env::current_dir`].
    #[doc(alias = "pushd")]
    pub fn push_dir<P: AsRef<Path>>(&self, path: P) -> PushDir<'_> {
        self._push_dir(path.as_ref())
    }
    fn _push_dir(&self, path: &Path) -> PushDir<'_> {
        let path = self.path(path);
        PushDir::new(self, path)
    }

    /// Fetches the environmental variable `key` for this [`Shell`].
    ///
    /// Returns an error if the variable is not set, or set to a non-utf8 value.
    ///
    /// Environment of the [`Shell`] affects all commands spawned via this
    /// shell.
    pub fn var<K: AsRef<OsStr>>(&self, key: K) -> Result<String> {
        self._var(key.as_ref())
    }
    fn _var(&self, key: &OsStr) -> Result<String> {
        match self._var_os(key) {
            Some(it) => it.into_string().map_err(VarError::NotUnicode),
            None => Err(VarError::NotPresent),
        }
        .map_err(|err| Error::new_var(err, key.to_os_string()))
    }

    /// Fetches the environmental variable `key` for this [`Shell`] as
    /// [`OsString`] Returns [`None`] if the variable is not set.
    ///
    /// Environment of the [`Shell`] affects all commands spawned via this
    /// shell.
    pub fn var_os<K: AsRef<OsStr>>(&self, key: K) -> Option<OsString> {
        self._var_os(key.as_ref())
    }
    fn _var_os(&self, key: &OsStr) -> Option<OsString> {
        self.env.borrow().get(key).cloned().or_else(|| env::var_os(key))
    }

    /// Sets the value of `key` environment variable for this [`Shell`] to
    /// `val`.
    ///
    /// Note that this doesn't affect [`std::env::var`].
    pub fn set_var<K: AsRef<OsStr>, V: AsRef<OsStr>>(&self, key: K, val: V) {
        self._set_var(key.as_ref(), val.as_ref())
    }
    fn _set_var(&self, key: &OsStr, val: &OsStr) {
        self.env.borrow_mut().insert(key.to_os_string(), val.to_os_string());
    }

    /// Temporary sets the value of `key` environment variable for this
    /// [`Shell`] to `val`.
    ///
    /// Returns a RAII guard which restores the old environment when dropped.
    ///
    /// Note that this doesn't affect [`std::env::var`].
    pub fn push_env<K: AsRef<OsStr>, V: AsRef<OsStr>>(&self, key: K, val: V) -> PushEnv<'_> {
        self._push_env(key.as_ref(), val.as_ref())
    }
    fn _push_env(&self, key: &OsStr, val: &OsStr) -> PushEnv<'_> {
        PushEnv::new(self, key.to_os_string(), val.to_os_string())
    }
    // endregion:env

    // region:fs
    /// Read the entire contents of a file into a string.
    #[doc(alias = "cat")]
    pub fn read_file<P: AsRef<Path>>(&self, path: P) -> Result<String> {
        self._read_file(path.as_ref())
    }
    fn _read_file(&self, path: &Path) -> Result<String> {
        let path = self.path(path);
        fs::read_to_string(&path).map_err(|err| Error::new_read_file(err, path))
    }

    /// Read the entire contents of a file into a vector of bytes.
    pub fn read_binary_file<P: AsRef<Path>>(&self, path: P) -> Result<Vec<u8>> {
        self._read_binary_file(path.as_ref())
    }
    fn _read_binary_file(&self, path: &Path) -> Result<Vec<u8>> {
        let path = self.path(path);
        fs::read(&path).map_err(|err| Error::new_read_file(err, path))
    }

    /// Returns a sorted list of paths directly contained in the directory at
    /// `path`.
    #[doc(alias = "ls")]
    pub fn read_dir<P: AsRef<Path>>(&self, path: P) -> Result<Vec<PathBuf>> {
        self._read_dir(path.as_ref())
    }
    fn _read_dir(&self, path: &Path) -> Result<Vec<PathBuf>> {
        let path = self.path(path);
        let mut res = Vec::new();
        || -> _ {
            for entry in fs::read_dir(&path)? {
                let entry = entry?;
                res.push(entry.path())
            }
            Ok(())
        }()
        .map_err(|err| Error::new_read_dir(err, path))?;
        res.sort();
        Ok(res)
    }

    /// Write a slice as the entire contents of a file.
    ///
    /// This function will create the file and all intermediate directories if
    /// they don't exist.
    // TODO: probably want to make this an atomic rename write?
    pub fn write_file<P: AsRef<Path>, C: AsRef<[u8]>>(&self, path: P, contents: C) -> Result<()> {
        self._write_file(path.as_ref(), contents.as_ref())
    }
    fn _write_file(&self, path: &Path, contents: &[u8]) -> Result<()> {
        let path = self.path(path);
        if let Some(p) = path.parent() {
            self.create_dir(p)?;
        }
        fs::write(&path, contents).map_err(|err| Error::new_write_file(err, path))
    }

    /// Copies `src` into `dst`.
    ///
    /// `src` must be a file, but `dst` need not be. If `dst` is an existing
    /// directory, `src` will be copied into a file in the `dst` directory whose
    /// name is same as that of `src`.
    ///
    /// Otherwise, `dst` is a file or does not exist, and `src` will be copied into
    /// it.
    #[doc(alias = "cp")]
    pub fn copy_file<S: AsRef<Path>, D: AsRef<Path>>(&self, src: S, dst: D) -> Result<()> {
        self._copy_file(src.as_ref(), dst.as_ref())
    }
    fn _copy_file(&self, src: &Path, dst: &Path) -> Result<()> {
        let src = self.path(src);
        let dst = self.path(dst);
        let dst = dst.as_path();
        let mut _tmp;
        let mut dst = dst;
        if dst.is_dir() {
            if let Some(file_name) = src.file_name() {
                _tmp = dst.join(file_name);
                dst = &_tmp;
            }
        }
        std::fs::copy(&src, dst)
            .map_err(|err| Error::new_copy_file(err, src.to_path_buf(), dst.to_path_buf()))?;
        Ok(())
    }

    /// Hardlinks `src` to `dst`.
    #[doc(alias = "ln")]
    pub fn hard_link<S: AsRef<Path>, D: AsRef<Path>>(&self, src: S, dst: D) -> Result<()> {
        self._hard_link(src.as_ref(), dst.as_ref())
    }
    fn _hard_link(&self, src: &Path, dst: &Path) -> Result<()> {
        let src = self.path(src);
        let dst = self.path(dst);
        fs::hard_link(&src, &dst).map_err(|err| Error::new_hard_link(err, src, dst))
    }

    /// Creates the specified directory.
    ///
    /// All intermediate directories will also be created.
    #[doc(alias("mkdir_p", "mkdir"))]
    pub fn create_dir<P: AsRef<Path>>(&self, path: P) -> Result<PathBuf> {
        self._create_dir(path.as_ref())
    }
    fn _create_dir(&self, path: &Path) -> Result<PathBuf> {
        let path = self.path(path);
        match fs::create_dir_all(&path) {
            Ok(()) => Ok(path),
            Err(err) => Err(Error::new_create_dir(err, path)),
        }
    }

    /// Creates an empty named world-readable temporary directory.
    ///
    /// Returns a [`TempDir`] RAII guard with the path to the directory. When
    /// dropped, the temporary directory and all of its contents will be
    /// removed.
    ///
    /// Note that this is an **insecure method** -- any other process on the
    /// system will be able to read the data.
    #[doc(alias = "mktemp")]
    pub fn create_temp_dir(&self) -> Result<TempDir> {
        let base = std::env::temp_dir();
        self.create_dir(&base)?;

        static CNT: AtomicUsize = AtomicUsize::new(0);

        let mut n_try = 0u32;
        loop {
            let cnt = CNT.fetch_add(1, Ordering::Relaxed);
            let path = base.join(format!("xshell-tmp-dir-{}", cnt));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(TempDir { path }),
                Err(err) if n_try == 1024 => return Err(Error::new_create_dir(err, path)),
                Err(_) => n_try += 1,
            }
        }
    }

    /// Removes the file or directory at the given path.
    #[doc(alias("rm_rf", "rm"))]
    pub fn remove_path<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        self._remove_path(path.as_ref())
    }
    fn _remove_path(&self, path: &Path) -> Result<(), Error> {
        let path = self.path(path);
        match path.metadata() {
            Ok(meta) => if meta.is_dir() { remove_dir_all(&path) } else { fs::remove_file(&path) }
                .map_err(|err| Error::new_remove_path(err, path)),
            Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
            Err(err) => Err(Error::new_remove_path(err, path)),
        }
    }

    /// Returns whether a file or directory exists at the given path.
    #[doc(alias("stat"))]
    pub fn path_exists<P: AsRef<Path>>(&self, path: P) -> bool {
        self.path(path.as_ref()).exists()
    }
    // endregion:fs

    /// Creates a new [`Cmd`] that executes the given `program`.
    pub fn cmd<P: AsRef<Path>>(&self, program: P) -> Cmd<'_> {
        // TODO: path lookup?
        Cmd::new(self, program.as_ref())
    }

    fn path(&self, p: &Path) -> PathBuf {
        let cd = self.cwd.borrow();
        cd.join(p)
    }

    /// Returns the environment variables set for this shell.
    ///
    /// The returned hash map contains a snapshot of the process’s environment variables at the time of this invocation. Modifications to environment variables afterwards will not be reflected in the returned iterator.
    pub fn vars_os(&self) -> Vec<(OsString, OsString)> {
        Vec::from_iter(self.env.borrow().iter().map(|(k, v)| (k.to_owned(), v.to_owned())))
    }

    /// Returns the environment variables set for this shell.
    ///
    /// The returned hash map contains a snapshot of the process’s environment
    /// variables at the time of this invocation. Modifications to environment
    /// variables afterwards will not be reflected in the returned iterator.
    ///
    /// # Panics
    ///
    /// While iterating, the returned iterator will panic if any key or value in
    /// the environment is not valid unicode. If this is not desired, consider
    /// using [`Shell::vars_os`](Self::vars_os).
    pub fn vars(&self) -> Vec<(String, String)> {
        Vec::from_iter(
            self.env
                .borrow()
                .iter()
                .map(|(k, v)| (k.to_str().unwrap().to_string(), v.to_str().unwrap().to_string())),
        )
    }
}

/// RAII guard returned from [`Shell::push_dir`].
///
/// Dropping `PushDir` restores the working directory of the [`Shell`] to the
/// old value.
#[derive(Debug)]
#[must_use]
pub struct PushDir<'a> {
    old_cwd: PathBuf,
    shell: &'a Shell,
}

impl<'a> PushDir<'a> {
    fn new(shell: &'a Shell, path: PathBuf) -> PushDir<'a> {
        PushDir { old_cwd: mem::replace(&mut *shell.cwd.borrow_mut(), path), shell }
    }
}

impl Drop for PushDir<'_> {
    fn drop(&mut self) {
        mem::swap(&mut *self.shell.cwd.borrow_mut(), &mut self.old_cwd)
    }
}

/// RAII guard returned from [`Shell::push_env`].
///
/// Dropping `PushEnv` restores the old value of the environmental variable.
#[derive(Debug)]
#[must_use]
pub struct PushEnv<'a> {
    key: OsString,
    old_value: Option<OsString>,
    shell: &'a Shell,
}

impl<'a> PushEnv<'a> {
    fn new(shell: &'a Shell, key: OsString, val: OsString) -> PushEnv<'a> {
        let old_value = shell.env.borrow_mut().insert(key.clone(), val);
        PushEnv { shell, key, old_value }
    }
}

impl Drop for PushEnv<'_> {
    fn drop(&mut self) {
        let mut env = self.shell.env.borrow_mut();
        let key = mem::take(&mut self.key);
        match self.old_value.take() {
            Some(value) => {
                env.insert(key, value);
            }
            None => {
                env.remove(&key);
            }
        }
    }
}

/// A builder object for constructing a subprocess.
///
/// A [`Cmd`] is usually created with the [`cmd!`] macro. The command exists
/// within a context of a [`Shell`] and uses its working directory and
/// environment.
///
/// # Example
///
/// ```no_run
/// use xshell::{Shell, cmd};
///
/// let sh = Shell::new()?;
///
/// let branch = "main";
/// let cmd = cmd!(sh, "git switch {branch}").quiet().run()?;
/// # Ok::<(), xshell::Error>(())
/// ```
#[derive(Debug)]
#[must_use]
pub struct Cmd<'a> {
    shell: &'a Shell,
    data: CmdData,
}

#[derive(Debug, Default, Clone)]
struct CmdData {
    prog: PathBuf,
    args: Vec<OsString>,
    env_changes: Vec<EnvChange>,
    ignore_status: bool,
    quiet: bool,
    secret: bool,
    stdin_contents: Option<Vec<u8>>,
    ignore_stdout: bool,
    ignore_stderr: bool,
}

// We just store a list of functions to call on the `Command` — the alternative
// would require mirroring the logic that `std::process::Command` (or rather
// `sys_common::CommandEnvs`) uses, which is moderately complex, involves
// special-casing `PATH`, and plausibly could change.
#[derive(Debug, Clone)]
enum EnvChange {
    Set(OsString, OsString),
    Remove(OsString),
    Clear,
}

impl fmt::Display for Cmd<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.data, f)
    }
}

impl fmt::Display for CmdData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.secret {
            return write!(f, "<secret>");
        }

        write!(f, "{}", self.prog.display())?;
        for arg in &self.args {
            // TODO: this is potentially not copy-paste safe.
            let arg = arg.to_string_lossy();
            if arg.chars().any(|it| it.is_ascii_whitespace()) {
                write!(f, " \"{}\"", arg.escape_default())?
            } else {
                write!(f, " {}", arg)?
            };
        }
        Ok(())
    }
}

impl From<Cmd<'_>> for Command {
    fn from(cmd: Cmd<'_>) -> Command {
        cmd.to_command()
    }
}

impl<'a> Cmd<'a> {
    fn new(shell: &'a Shell, prog: &Path) -> Cmd<'a> {
        let mut data = CmdData::default();
        data.prog = prog.to_path_buf();
        Cmd { shell, data }
    }

    // region:builder
    /// Adds an argument to this commands.
    pub fn arg<P: AsRef<OsStr>>(mut self, arg: P) -> Cmd<'a> {
        self._arg(arg.as_ref());
        self
    }
    fn _arg(&mut self, arg: &OsStr) {
        self.data.args.push(arg.to_owned())
    }

    /// Adds all of the arguments to this command.
    pub fn args<I>(mut self, args: I) -> Cmd<'a>
    where
        I: IntoIterator,
        I::Item: AsRef<OsStr>,
    {
        args.into_iter().for_each(|it| self._arg(it.as_ref()));
        self
    }

    #[doc(hidden)]
    pub fn __extend_arg<P: AsRef<OsStr>>(mut self, arg_fragment: P) -> Cmd<'a> {
        self.___extend_arg(arg_fragment.as_ref());
        self
    }
    fn ___extend_arg(&mut self, arg_fragment: &OsStr) {
        match self.data.args.last_mut() {
            Some(last_arg) => last_arg.push(arg_fragment),
            None => {
                let mut prog = mem::take(&mut self.data.prog).into_os_string();
                prog.push(arg_fragment);
                self.data.prog = prog.into();
            }
        }
    }

    /// Overrides the value of the environmental variable for this command.
    pub fn env<K: AsRef<OsStr>, V: AsRef<OsStr>>(mut self, key: K, val: V) -> Cmd<'a> {
        self._env_set(key.as_ref(), val.as_ref());
        self
    }

    fn _env_set(&mut self, key: &OsStr, val: &OsStr) {
        self.data.env_changes.push(EnvChange::Set(key.to_owned(), val.to_owned()));
    }

    /// Overrides the values of specified environmental variables for this
    /// command.
    pub fn envs<I, K, V>(mut self, vars: I) -> Cmd<'a>
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        vars.into_iter().for_each(|(k, v)| self._env_set(k.as_ref(), v.as_ref()));
        self
    }

    /// Removes the environment variable from this command.
    pub fn env_remove<K: AsRef<OsStr>>(mut self, key: K) -> Cmd<'a> {
        self._env_remove(key.as_ref());
        self
    }
    fn _env_remove(&mut self, key: &OsStr) {
        self.data.env_changes.push(EnvChange::Remove(key.to_owned()));
    }

    /// Removes all of the environment variables from this command.
    pub fn env_clear(mut self) -> Cmd<'a> {
        self.data.env_changes.push(EnvChange::Clear);
        self
    }

    /// Don't return an error if command the command exits with non-zero status.
    ///
    /// By default, non-zero exit status is considered an error.
    pub fn ignore_status(mut self) -> Cmd<'a> {
        self.set_ignore_status(true);
        self
    }
    /// Controls whether non-zero exit status is considered an error.
    pub fn set_ignore_status(&mut self, yes: bool) {
        self.data.ignore_status = yes;
    }

    /// Don't echo the command itself to stderr.
    ///
    /// By default, the command itself will be printed to stderr when executed via [`Cmd::run`].
    pub fn quiet(mut self) -> Cmd<'a> {
        self.set_quiet(true);
        self
    }
    /// Controls whether the command itself is printed to stderr.
    pub fn set_quiet(&mut self, yes: bool) {
        self.data.quiet = yes;
    }

    /// Marks the command as secret.
    ///
    /// If a command is secret, it echoes `<secret>` instead of the program and
    /// its arguments, even in error messages.
    pub fn secret(mut self) -> Cmd<'a> {
        self.set_secret(true);
        self
    }
    /// Controls whether the command is secret.
    pub fn set_secret(&mut self, yes: bool) {
        self.data.secret = yes;
    }

    /// Pass the given slice to the standard input of the spawned process.
    pub fn stdin(mut self, stdin: impl AsRef<[u8]>) -> Cmd<'a> {
        self._stdin(stdin.as_ref());
        self
    }
    fn _stdin(&mut self, stdin: &[u8]) {
        self.data.stdin_contents = Some(stdin.to_vec());
    }

    /// Ignores the standard output stream of the process.
    ///
    /// This is equivalent to redirecting stdout to `/dev/null`. By default, the
    /// stdout is inherited or captured.
    pub fn ignore_stdout(mut self) -> Cmd<'a> {
        self.set_ignore_stdout(true);
        self
    }
    /// Controls whether the standard output is ignored.
    pub fn set_ignore_stdout(&mut self, yes: bool) {
        self.data.ignore_stdout = yes;
    }

    /// Ignores the standard output stream of the process.
    ///
    /// This is equivalent redirecting stderr to `/dev/null`. By default, the
    /// stderr is inherited or captured.
    pub fn ignore_stderr(mut self) -> Cmd<'a> {
        self.set_ignore_stderr(true);
        self
    }
    /// Controls whether the standard error is ignored.
    pub fn set_ignore_stderr(&mut self, yes: bool) {
        self.data.ignore_stderr = yes;
    }
    // endregion:builder

    // region:running
    /// Runs the command.
    ///
    /// By default the command itself is echoed to stderr, its standard streams
    /// are inherited, and non-zero return code is considered an error. These
    /// behaviors can be overridden by using various builder methods of the [`Cmd`].
    pub fn run(&self) -> Result<()> {
        if !self.data.quiet {
            eprintln!("$ {}", self);
        }
        self.output_impl(false, false).map(|_| ())
    }

    /// Run the command and return its stdout as a string.
    pub fn read(&self) -> Result<String> {
        self.read_stream(false)
    }

    /// Run the command and return its stderr as a string.
    pub fn read_stderr(&self) -> Result<String> {
        self.read_stream(true)
    }

    /// Run the command and return its output.
    pub fn output(&self) -> Result<Output> {
        self.output_impl(true, true)
    }
    // endregion:running

    fn read_stream(&self, read_stderr: bool) -> Result<String> {
        let read_stdout = !read_stderr;
        let output = self.output_impl(read_stdout, read_stderr)?;
        self.check_status(output.status)?;

        let stream = if read_stderr { output.stderr } else { output.stdout };
        let mut stream = String::from_utf8(stream).map_err(|err| Error::new_cmd_utf8(self, err))?;

        if stream.ends_with('\n') {
            stream.pop();
        }
        if stream.ends_with('\r') {
            stream.pop();
        }

        Ok(stream)
    }

    fn output_impl(&self, read_stdout: bool, read_stderr: bool) -> Result<Output> {
        let mut child = {
            let mut command = self.to_command();

            if !self.data.ignore_stdout {
                command.stdout(if read_stdout { Stdio::piped() } else { Stdio::inherit() });
            }
            if !self.data.ignore_stderr {
                command.stderr(if read_stderr { Stdio::piped() } else { Stdio::inherit() });
            }

            command.stdin(match &self.data.stdin_contents {
                Some(_) => Stdio::piped(),
                None => Stdio::null(),
            });

            command.spawn().map_err(|err| {
                // Try to determine whether the command failed because the current
                // directory does not exist. Return an appropriate error in such a
                // case.
                if matches!(err.kind(), io::ErrorKind::NotFound) {
                    let cwd = self.shell.cwd.borrow();
                    if let Err(err) = cwd.metadata() {
                        return Error::new_current_dir(err, Some(cwd.clone()));
                    }
                }
                Error::new_cmd_io(self, err)
            })?
        };

        let mut io_thread = None;
        if let Some(stdin_contents) = self.data.stdin_contents.clone() {
            let mut stdin = child.stdin.take().unwrap();
            io_thread = Some(std::thread::spawn(move || {
                stdin.write_all(&stdin_contents)?;
                stdin.flush()
            }));
        }
        let out_res = child.wait_with_output();
        let err_res = io_thread.map(|it| it.join().unwrap());
        let output = out_res.map_err(|err| Error::new_cmd_io(self, err))?;
        if let Some(err_res) = err_res {
            err_res.map_err(|err| Error::new_cmd_stdin(self, err))?;
        }
        self.check_status(output.status)?;
        Ok(output)
    }

    fn to_command(&self) -> Command {
        let mut res = Command::new(&self.data.prog);
        res.current_dir(self.shell.current_dir());
        res.args(&self.data.args);

        for (key, val) in &*self.shell.env.borrow() {
            res.env(key, val);
        }
        for change in &self.data.env_changes {
            match change {
                EnvChange::Clear => res.env_clear(),
                EnvChange::Remove(key) => res.env_remove(key),
                EnvChange::Set(key, val) => res.env(key, val),
            };
        }

        if self.data.ignore_stdout {
            res.stdout(Stdio::null());
        }

        if self.data.ignore_stderr {
            res.stderr(Stdio::null());
        }

        res
    }

    fn check_status(&self, status: ExitStatus) -> Result<()> {
        if status.success() || self.data.ignore_status {
            return Ok(());
        }
        Err(Error::new_cmd_status(self, status))
    }
}

/// A temporary directory.
///
/// This is a RAII object which will remove the underlying temporary directory
/// when dropped.
#[derive(Debug)]
#[must_use]
pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    /// Returns the path to the underlying temporary directory.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = remove_dir_all(&self.path);
    }
}

#[cfg(not(windows))]
fn remove_dir_all(path: &Path) -> io::Result<()> {
    std::fs::remove_dir_all(path)
}

#[cfg(windows)]
fn remove_dir_all(path: &Path) -> io::Result<()> {
    for _ in 0..99 {
        if fs::remove_dir_all(path).is_ok() {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(10))
    }
    fs::remove_dir_all(path)
}
