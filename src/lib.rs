//! xshell is a swiss-army knife for writing cross-platform "bash" scripts in Rust.
//!
//! It doesn't use the shell directly, but rather re-implements parts of scripting environment in
//! Rust. The intended use-case is various bits of glue code, which could be written in bash or
//! python. The original motivation is [`xtask`](https://github.com/matklad/cargo-xtask)
//! development.
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
//! * Ergonomics and DWIM ("do what I mean"): `cmd!` macro supports interpolation, writing to a file
//!   automatically creates parent directories, etc.
//! * Reliability: no [shell injection] by construction, good error messages with file paths,
//!   non-zero exit status is an error, consistent behavior across platforms, etc.
//! * Frugality: fast compile times, few dependencies, low-tech API.
//!
//! ## Guide
//!
//! For a short API overview, let's implement a script to clone a github repository and publish it
//! as a crates.io crate. The script will do the following:
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
//! Only two imports are needed -- the [`Shell`] struct the and [`cmd!`] macro. By convention, an
//! instance of a [`Shell`] is stored in a variable named `sh`. All the API is available as methods,
//! so a short name helps here. For "scripts", the [`anyhow`](https://docs.rs/anyhow) crate is a
//! great choice for an error-handling library.
//!
//! Next, clone the repository:
//!
//! ```no_run
//! # use xshell::{Shell, cmd}; let sh = Shell::new().unwrap();
//! cmd!(sh, "git clone https://github.com/matklad/xshell.git").run_echo()?;
//! # Ok::<(), xshell::Error>(())
//! ```
//!
//! The [`cmd!`] macro provides a convenient syntax for creating a command -- the [`Cmd`] struct.
//! The [`Cmd::run_echo`] method runs the command as if you typed it into the shell. The whole
//! program outputs:
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
//! Note that the command itself is echoed to stderr (the `$ git ...` bit in the output).
//!
//! Printing command itself and its output is a good default for "interactive" scripts where the
//! user watches the output "live". For batch scripts, where the output is only relevant if an error
//! occurs, there's run method:
//!
//! ```no_run
//! # use xshell::{Shell, cmd}; let sh = Shell::new().unwrap();
//! cmd!(sh, "git clone https://github.com/matklad/xshell.git")
//!     .run()?;
//! # Ok::<(), xshell::Error>(())
//! ```
//!
//! To make the code more general, let's use command interpolation to extract the username and the
//! repository:
//!
//! ```no_run
//! # use xshell::{Shell, cmd}; let sh = Shell::new().unwrap();
//! let user = "matklad";
//! let repo = "xshell";
//! cmd!(sh, "git clone https://github.com/{user}/{repo}.git").run_echo()?;
//! # Ok::<(), xshell::Error>(())
//! ```
//!
//! Note that the `cmd!` macro parses the command string at compile time, so you don't have to worry
//! about escaping the arguments. For example, the following command "touches" a single file whose
//! name is `contains a space`:
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
//! # use xshell::{Shell, cmd}; let mut sh = Shell::new().unwrap();
//! # let repo = "xshell";
//! sh.set_current_dir(repo);
//! ```
//!
//! Each instance of [`Shell`] has a current directory, which is independent of the process-wide
//! [`std::env::current_dir`]. The same applies to the environment.
//!
//! Next, run the tests:
//!
//! ```no_run
//! # use xshell::{Shell, cmd}; let sh = Shell::new().unwrap();
//! let test_args = ["-Zunstable-options", "--report-time"];
//! cmd!(sh, "cargo test -- {test_args...}").run_echo()?;
//! # Ok::<(), xshell::Error>(())
//! ```
//!
//! Note how the so-called splat syntax (`...`) is used to interpolate an iterable of arguments.
//!
//! Next, read the Cargo.toml so that we can fetch crate' declared version:
//!
//! ```no_run
//! # use xshell::{Shell, cmd}; let sh = Shell::new().unwrap();
//! let manifest = sh.read_file("Cargo.toml")?;
//! # Ok::<(), xshell::Error>(())
//! ```
//!
//! [`Shell::read_file`] works like [`std::fs::read_to_string`], but paths are relative to the
//! current directory of the [`Shell`]. Unlike [`std::fs`], error messages are much more useful. For
//! example, if there isn't a `Cargo.toml` in the repository, the error message is:
//!
//! ```text
//! Error: failed to read file `xshell/Cargo.toml`: no such file or directory (os error 2)
//! ```
//!
//! `xshell` doesn't implement string processing utils like `grep`, `sed` or `awk` -- there's no
//! need to, built-in language features work fine, and it's always possible to pull extra
//! functionality from crates.io.
//!
//! To extract the `version` field from Cargo.toml, [`str::split_once`] is enough:
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
//! cmd!(sh, "git tag {version}").run_echo()?;
//! # Ok::<(), anyhow::Error>(())
//! ```
//!
//! The splat (`...`) syntax works with any iterable, and in Rust options are iterable. This means
//! that `...` can be used to implement optional arguments. For example, here's how to pass
//! `--dry-run` when *not* running in CI:
//!
//! ```no_run
//! # use xshell::{Shell, cmd}; let sh = Shell::new().unwrap();
//! let dry_run = if sh.var("CI").is_ok() { None } else { Some("--dry-run") };
//! cmd!(sh, "cargo publish {dry_run...}").run_echo()?;
//! # Ok::<(), xshell::Error>(())
//! ```
//!
//! Putting everything altogether, here's the whole script:
//!
//! ```no_run
//! use xshell::{cmd, Shell};
//!
//! fn main() -> anyhow::Result<()> {
//!     let mut sh = Shell::new()?;
//!
//!     let user = "matklad";
//!     let repo = "xshell";
//!     cmd!(sh, "git clone https://github.com/{user}/{repo}.git").run()?;
//!     sh.set_current_dir(repo);
//!
//!     let test_args = ["-Zunstable-options", "--report-time"];
//!     cmd!(sh, "cargo test -- {test_args...}").run_echo()?;
//!
//!     let manifest = sh.read_file("Cargo.toml")?;
//!     let version = manifest
//!         .split_once("version = \"")
//!         .and_then(|it| it.1.split_once('\"'))
//!         .map(|it| it.0)
//!         .ok_or_else(|| anyhow::format_err!("can't find version field in the manifest"))?;
//!
//!     cmd!(sh, "git tag {version}").run_echo()?;
//!
//!     let dry_run = if sh.var("CI").is_ok() { None } else { Some("--dry-run") };
//!     cmd!(sh, "cargo publish {dry_run...}").run()?;
//!
//!     Ok(())
//! }
//! ```
//!
//! `xshell` itself uses a similar script to automatically publish oneself to crates.io when the
//! version in Cargo.toml changes:
//!
//! <https://github.com/matklad/xshell/blob/master/examples/ci.rs>
//!
//! ## Maintenance
//!
//! MSRV bump is not considered semver breaking. MSRV is updated conservatively.
//!
//! The crate isn't comprehensive yet, but this is a goal. You are hereby encouraged to submit PRs
//! with missing functionality!
//!
//! # Related Crates
//!
//! [`duct`] is a crate for heavy-duty process herding, with support for pipelines.
//!
//! Most of what this crate provides can be open-coded using [`std::process::Command`] and
//! [`std::fs`]. If you only need to spawn a single process, using `std` is probably better (but
//! don't forget to check the exit status!).
//!
//! [`duct`]: https://github.com/oconnor663/duct.rs
//! [shell injection]: https://en.wikipedia.org/wiki/Code_injection#Shell_injection
//!
//! The [`dax`](https://github.com/dsherret/dax) library for Deno shares the overall philosophy with
//! `xshell`, but is much more thorough and complete. If you don't need Rust, use `dax`.
//!
//! ## Implementation Notes
//!
//! The design is heavily inspired by the Julia language:
//!
//! * [Shelling Out Sucks](https://julialang.org/blog/2012/03/shelling-out-sucks/)
//! * [Put This In Your Pipe](https://julialang.org/blog/2013/04/put-this-in-your-pipe/)
//! * [Running External
//!   Programs](https://docs.julialang.org/en/v1/manual/running-external-programs/)
//! * [Filesystem](https://docs.julialang.org/en/v1/base/file/)
//!
//! Smaller influences are the [`duct`] crate and Ruby's
//! [`FileUtils`](https://ruby-doc.org/stdlib-2.4.1/libdoc/fileutils/rdoc/FileUtils.html) module.
//!
//! The `cmd!` macro uses a simple proc-macro internally. It doesn't depend on helper libraries, so
//! the fixed-cost impact on compile times is moderate. Compiling a trivial program with `cmd!("date
//! +%Y-%m-%d")` takes one second. Equivalent program using only `std::process::Command` compiles in
//! 0.25 seconds.
//!
//! To make IDEs infer correct types without expanding proc-macro, it is wrapped into a declarative
//! macro which supplies type hints.

#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![deny(rust_2018_idioms)]

mod exec;
mod error;

use std::{
    collections::HashMap,
    env::{self, current_dir, VarError},
    ffi::{OsStr, OsString},
    fmt::{self},
    fs,
    io::{self, ErrorKind},
    mem,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

pub use crate::error::{Error, Result};
use error::CmdErrorKind;
#[doc(hidden)]
pub use xshell_macros::__cmd;

const STREAM_SUFFIX_SIZE: usize = 128 * 1024; // 128KiB

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
/// // Like in the shell, single quotes prevent interpolation:
/// let c = cmd!(sh, "echo 'spaces '{greeting}' around {greeting}'");
/// assert_eq!(c.to_string(), r#"echo "spaces hello world around {greeting}""#);
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
        #[cfg(any())] // Trick rust analyzer into highlighting interpolated bits
        format_args!($cmd);
        let f = |prog| $sh.cmd(prog);
        let cmd: $crate::Cmd = $crate::__cmd!(f $cmd);
        cmd
    }};
}

/// A `Shell` is the main API entry point.
///
/// Almost all of the crate's functionality is available as methods of the `Shell` object.
///
/// `Shell` is a stateful object. It maintains a logical working directory and an environment map.
/// They are independent from process's [`std::env::current_dir`] and [`std::env::var`], and only
/// affect paths and commands passed to the [`Shell`]. `Shell` is cheaply clonable and you can use
/// methods like [`Shell::with_current_dir`] to create independent copies with separate
/// environments.
///
/// By convention, the variable holding the shell is named `sh`.
///
/// # Example
///
/// ```no_run
/// use xshell::{cmd, Shell};
///
/// let sh = Shell::new()?;
/// let sh = sh.with_current_dir("./target");
/// let cwd = sh.current_dir();
/// cmd!(sh, "echo current dir is {cwd}").run()?;
///
/// let process_cwd = std::env::current_dir().unwrap();
/// assert_eq!(cwd, process_cwd.join("./target"));
/// # Ok::<(), xshell::Error>(())
/// ```
#[derive(Debug, Clone)]
pub struct Shell {
    cwd: Arc<Path>,
    env: Arc<HashMap<Arc<OsStr>, Arc<OsStr>>>,
}

impl Shell {
    /// Creates a new [`Shell`].
    ///
    /// Fails if [`std::env::current_dir`] returns an error.
    pub fn new() -> Result<Shell> {
        let cwd = current_dir().map_err(|err| Error::new_current_dir(err, None))?;
        Ok(Shell { cwd: cwd.into(), env: Default::default() })
    }

    /// Returns the working directory for this [`Shell`].
    ///
    /// All relative paths are interpreted relative to this directory, rather
    /// than [`std::env::current_dir`].
    #[doc(alias = "pwd")]
    pub fn current_dir(&self) -> &Path {
        self.cwd.as_ref()
    }

    /// Changes the working directory for this [`Shell`].
    ///
    /// Note that this doesn't affect [`std::env::current_dir`].
    #[doc(alias = "cd")]
    pub fn set_current_dir(&mut self, path: impl AsRef<Path>) {
        fn inner(sh: &mut Shell, path: &OsStr) {
            sh.cwd = sh.cwd.join(path).into();
        }
        inner(self, path.as_ref().as_os_str());
    }

    /// Returns a new [`Shell`] with the working directory set to `path`.
    ///
    /// Note that this doesn't affect [`std::env::current_dir`].
    #[doc(alias = "pushd")]
    #[must_use]
    pub fn with_current_dir(&self, path: impl AsRef<Path>) -> Shell {
        fn inner(sh: &Shell, path: &OsStr) -> Shell {
            Shell { cwd: sh.cwd.join(path).into(), env: sh.env.clone() }
        }
        inner(self, path.as_ref().as_os_str())
    }

    /// Fetches the environmental variable `key` for this [`Shell`].
    ///
    /// Returns an error if the variable is not set, or set to a non-utf8 value.
    ///
    /// Environment of the [`Shell`] affects all commands spawned via this
    /// shell.
    pub fn var(&self, key: impl AsRef<OsStr>) -> Result<String> {
        fn inner(sh: &Shell, key: &OsStr) -> Result<String> {
            let env_os = sh
                .var_os(key)
                .ok_or(VarError::NotPresent)
                .map_err(|err| Error::new_var(err, key.to_os_string()))?;
            env_os
                .into_string()
                .map_err(|value| Error::new_var(VarError::NotUnicode(value), key.to_os_string()))
        }
        inner(self, key.as_ref())
    }

    /// Fetches the environmental variable `key` for this [`Shell`] as
    /// [`OsString`] Returns [`None`] if the variable is not set.
    ///
    /// Environment of the [`Shell`] affects all commands spawned via this
    /// shell.
    pub fn var_os(&self, key: impl AsRef<OsStr>) -> Option<OsString> {
        fn inner(sh: &Shell, key: &OsStr) -> Option<OsString> {
            sh.env.get(key).map(OsString::from).or_else(|| env::var_os(key))
        }
        inner(self, key.as_ref())
    }

    /// Fetches the whole environment as a `(Key, Value)` iterator for this [`Shell`].
    ///
    /// Returns an error if any of the variables are not utf8.
    ///
    /// Environment of the [`Shell`] affects all commands spawned via this
    /// shell.
    pub fn vars_os(&self) -> HashMap<OsString, OsString> {
        let mut result: HashMap<OsString, OsString> = Default::default();
        result.extend(env::vars_os());
        result.extend(self.env.iter().map(|(k, v)| (OsString::from(k), OsString::from(v))));
        result
    }

    /// Sets the value of `key` environment variable for this [`Shell`] to `value`.
    ///
    /// Note that this doesn't affect [`std::env::var`].
    pub fn set_var(&mut self, key: impl AsRef<OsStr>, value: impl AsRef<OsStr>) {
        fn inner(sh: &mut Shell, key: &OsStr, value: &OsStr) {
            Arc::make_mut(&mut sh.env).insert(key.into(), value.into());
        }
        inner(self, key.as_ref(), value.as_ref());
    }

    /// Returns a new [`Shell`] with environmental variable `key` set to `value`.
    ///
    /// Note that this doesn't affect [`std::env::var`].
    pub fn with_var(&self, key: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> Shell {
        fn inner(sh: &Shell, key: &OsStr, value: &OsStr) -> Shell {
            let mut env = Arc::clone(&sh.env);
            Arc::make_mut(&mut env).insert(key.into(), value.into());
            Shell { cwd: sh.cwd.clone(), env }
        }
        inner(self, key.as_ref(), value.as_ref())
    }

    /// Read an utf-8 encoded text file into string.
    #[doc(alias = "cat")]
    pub fn read_file(&self, path: impl AsRef<Path>) -> Result<String> {
        fn inner(sh: &Shell, path: &Path) -> Result<String> {
            let path = sh.path(path);
            fs::read_to_string(&path).map_err(|err| Error::new_read_file(err, path))
        }
        inner(self, path.as_ref())
    }

    /// Read a file into a vector of bytes.
    pub fn read_binary_file(&self, path: impl AsRef<Path>) -> Result<Vec<u8>> {
        fn inner(sh: &Shell, path: &Path) -> Result<Vec<u8>> {
            let path = sh.path(path);
            fs::read(&path).map_err(|err| Error::new_read_file(err, path))
        }
        inner(self, path.as_ref())
    }

    /// Write a slice as the entire contents of a file.
    ///
    /// This function will create the file and all intermediate directories if
    /// they don't exist.
    // TODO: probably want to make this an atomic rename write?
    pub fn write_file(&self, path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> Result<()> {
        fn inner(sh: &Shell, path: &Path, contents: &[u8]) -> Result<()> {
            let path = sh.path(path);
            if let Some(p) = path.parent() {
                sh.create_dir(p)?;
            }
            fs::write(&path, contents).map_err(|err| Error::new_write_file(err, path))
        }
        inner(self, path.as_ref(), contents.as_ref())
    }

    /// Creates a `dst` file with the same contents as `src`
    #[doc(alias = "cp")]
    pub fn copy_file(&self, src_file: impl AsRef<Path>, dst_file: impl AsRef<Path>) -> Result<()> {
        fn inner(sh: &Shell, src: &Path, dst: &Path) -> Result<()> {
            let src = sh.path(src);
            let dst = sh.path(dst);
            if let Some(p) = dst.parent() {
                sh.create_dir(p)?;
            }
            std::fs::copy(&src, &dst)
                .map_err(|err| Error::new_copy_file(err, src.to_path_buf(), dst.to_path_buf()))?;
            Ok(())
        }
        inner(self, src_file.as_ref(), dst_file.as_ref())
    }

    /// Creates a file in `dst` directory with the same name and contents as `src`.
    #[doc(alias = "cp")]
    pub fn copy_file_to_dir(
        &self,
        src_file: impl AsRef<Path>,
        dst_dir: impl AsRef<Path>,
    ) -> Result<()> {
        fn inner(sh: &Shell, src: &Path, dst: &Path) -> Result<()> {
            let src = sh.path(src);
            let dst = sh.path(dst);
            let Some(file_name) = src.file_name() else {
                return Err(Error::new_copy_file(io::ErrorKind::InvalidData.into(), src, dst));
            };
            sh.copy_file(&src, &dst.join(file_name))
        }
        inner(self, src_file.as_ref(), dst_dir.as_ref())
    }

    /// Hardlinks `src` to `dst`.
    #[doc(alias = "ln")]
    pub fn hard_link(&self, src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<()> {
        fn inner(sh: &Shell, src: &Path, dst: &Path) -> Result<()> {
            let src = sh.path(src);
            let dst = sh.path(dst);
            fs::hard_link(&src, &dst).map_err(|err| Error::new_hard_link(err, src, dst))
        }
        inner(self, src.as_ref(), dst.as_ref())
    }

    /// Returns a sorted list of paths directly contained in the directory at `path`.
    #[doc(alias = "ls")]
    pub fn read_dir(&self, path: impl AsRef<Path>) -> Result<Vec<PathBuf>> {
        fn inner(sh: &Shell, path: &Path) -> Result<Vec<PathBuf>> {
            let path = sh.path(path);
            let mut res = Vec::new();
            || -> _ {
                for entry in fs::read_dir(&path)? {
                    let entry = entry?;
                    res.push(entry.path())
                }
                Ok(())
            }()
            .map_err(|err| Error::new_read_dir(err, path))?;
            // Sort to ensure determinism, and ease debugging of downstream programs!
            res.sort();
            Ok(res)
        }

        inner(self, path.as_ref())
    }

    /// Ensures that the specified directory exist.
    ///
    /// All intermediate directories will also be created as needed.
    #[doc(alias("mkdir_p", "mkdir"))]
    pub fn create_dir<P: AsRef<Path>>(&self, path: P) -> Result<PathBuf> {
        fn inner(sh: &Shell, path: &Path) -> Result<PathBuf> {
            let path = sh.path(path);
            match fs::create_dir_all(&path) {
                Ok(()) => Ok(path),
                Err(err) => Err(Error::new_create_dir(err, path)),
            }
        }
        inner(self, path.as_ref())
    }

    /// Creates an empty named world-readable temporary directory.
    ///
    /// Returns a [`TempDir`] RAII guard with the path to the directory. When dropped, the temporary
    /// directory and all of its contents will be removed.
    ///
    /// Note that this is an **insecure method** -- any other process on the system will be able to
    /// read the data.
    #[doc(alias = "mktemp")]
    pub fn create_temp_dir(&self) -> Result<TempDir> {
        let base = std::env::temp_dir();
        self.create_dir(&base)?;

        static CNT: AtomicUsize = AtomicUsize::new(0);

        // TODO: once std gets random numbers, start with random u128 here.
        let mut try_count = 0u32;
        loop {
            let cnt = CNT.fetch_add(1, Ordering::Relaxed);
            let path = base.join(format!("xshell-tmp-dir-{}", cnt));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(TempDir { path }),
                Err(err) if try_count == 1024 => return Err(Error::new_create_dir(err, path)),
                Err(_) => try_count += 1,
            }
        }
    }

    /// Removes the file or directory at the given path.
    #[doc(alias("rm_rf", "rm"))]
    pub fn remove_path(&self, path: impl AsRef<Path>) -> Result<()> {
        fn inner(sh: &Shell, path: &Path) -> Result<(), Error> {
            let path = sh.path(path);
            match path.metadata() {
                Ok(meta) => {
                    if meta.is_dir() { remove_dir_all(&path) } else { fs::remove_file(&path) }
                        .map_err(|err| Error::new_remove_path(err, path))
                }
                Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
                Err(err) => Err(Error::new_remove_path(err, path)),
            }
        }
        inner(self, path.as_ref())
    }

    /// Returns whether a file or directory exists at the given path.
    ///
    /// Be mindful of Time Of Check, Time Of Use (TOCTOU) errors -- often, it is better to attempt a
    /// given operation and handle an error if a path doesn't exist, instead of trying to check
    /// beforehand.
    #[doc(alias("stat"))]
    pub fn path_exists<P: AsRef<Path>>(&self, path: P) -> bool {
        self.path(path.as_ref()).exists()
    }

    /// Creates a new [`Cmd`] that executes the given `program`.
    pub fn cmd(&self, program: impl AsRef<OsStr>) -> Cmd {
        // TODO: path lookup?
        Cmd::new(self, program.as_ref())
    }

    fn path(&self, p: &Path) -> PathBuf {
        self.cwd.join(p)
    }
}

/// A builder object for constructing a subprocess.
///
/// A [`Cmd`] is usually created with the [`cmd!`] macro. The command exists within a context of a
/// [`Shell`] and uses its working directory and environment.
///
/// # Example
///
/// ```no_run
/// use xshell::{Shell, cmd};
///
/// let sh = Shell::new()?;
///
/// let branch = "main";
/// let cmd = cmd!(sh, "git switch {branch}").run()?;
/// # Ok::<(), xshell::Error>(())
/// ```
///
/// Use:
///
/// * [`Cmd::run_echo`] for interactive scripts where the user watches the output live.
/// * [`Cmd::run`] for batch scripts where the output matters only if an error occurs.
/// * [`Cmd::read`] to get command's output.
///
/// Methods for fine-grained control over child process stdio are intentionally not provided. If you
/// need anything not covered by `Cmd` API, use [`Cmd::to_command`] to convert it to
/// [`std::process::Command`].
#[derive(Debug, Clone)]
#[must_use]
pub struct Cmd {
    sh: Shell,
    prog: PathBuf,
    args: Vec<OsString>,
    stdin_contents: Option<Vec<u8>>,
    deadline: Option<Instant>,
    ignore_status: bool,
    secret: bool,
}

impl fmt::Display for Cmd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.secret {
            return write!(f, "<secret>");
        }

        write!(f, "{}", self.prog.as_path().display())?;
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

impl From<Cmd> for Command {
    fn from(cmd: Cmd) -> Command {
        cmd.to_command()
    }
}

impl Cmd {
    fn new(sh: &Shell, program: impl AsRef<Path>) -> Cmd {
        fn inner(sh: &Shell, program: &Path) -> Cmd {
            Cmd {
                sh: sh.clone(),
                prog: program.into(),
                args: Vec::new(),
                stdin_contents: None,
                ignore_status: false,
                deadline: None,
                secret: false,
            }
        }
        inner(sh, program.as_ref())
    }

    /// Adds an argument to this command.
    pub fn arg(mut self, arg: impl AsRef<OsStr>) -> Cmd {
        self.arg_inner(arg.as_ref());
        self
    }
    fn arg_inner(&mut self, arg: &OsStr) {
        self.args.push(arg.to_owned())
    }

    /// Adds all of the arguments to this command.
    pub fn args<I>(mut self, args: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<OsStr>,
    {
        args.into_iter().for_each(|it| self.arg_inner(it.as_ref()));
        self
    }

    #[doc(hidden)]
    pub fn __extend_arg(mut self, arg_fragment: impl AsRef<OsStr>) -> Cmd {
        fn inner(sh: &mut Cmd, arg_fragment: &OsStr) {
            match sh.args.last_mut() {
                Some(last_arg) => last_arg.push(arg_fragment),
                None => {
                    let mut inner = mem::take(&mut sh.prog).into_os_string();
                    inner.push(arg_fragment);
                    sh.prog = inner.into();
                }
            }
        }
        inner(&mut self, arg_fragment.as_ref());
        self
    }

    /// Overrides the value of the environmental variable for this command.
    pub fn env(mut self, key: impl AsRef<OsStr>, val: impl AsRef<OsStr>) -> Cmd {
        fn inner(sh: &mut Cmd, key: &OsStr, val: &OsStr) {
            Arc::make_mut(&mut sh.sh.env).insert(key.into(), val.into());
        }
        inner(&mut self, key.as_ref(), val.as_ref());
        self
    }

    /// Overrides the values of specified environmental variables for this command.
    pub fn envs<I, K, V>(mut self, vars: I) -> Cmd
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        Arc::make_mut(&mut self.sh.env)
            .extend(vars.into_iter().map(|(k, v)| (k.as_ref().into(), v.as_ref().into())));
        self
    }

    /// Removes the environment variable from this command.
    pub fn env_remove(mut self, key: impl AsRef<OsStr>) -> Cmd {
        fn inner(sh: &mut Cmd, key: &OsStr) {
            Arc::make_mut(&mut sh.sh.env).remove(key);
        }
        inner(&mut self, key.as_ref());
        self
    }

    /// Removes all of the environment variables from this command.
    pub fn env_clear(mut self) -> Cmd {
        Arc::make_mut(&mut self.sh.env).clear();
        self
    }

    /// Pass the given slice to the standard input of the spawned process.
    pub fn stdin(mut self, stdin: impl AsRef<[u8]>) -> Cmd {
        fn inner(sh: &mut Cmd, stdin: &[u8]) {
            sh.stdin_contents = Some(stdin.to_vec());
        }
        inner(&mut self, stdin.as_ref());
        self
    }

    /// Don't return an error if the command doesn't exit with status zero.
    pub fn ignore_status(mut self) -> Cmd {
        self.set_ignore_status(true);
        self
    }

    /// Whether to return an error if the command doesn't exit with status zero.
    pub fn set_ignore_status(&mut self, yes: bool) {
        self.ignore_status = yes;
    }

    /// Set timeout.
    pub fn timeout(mut self, timeout: Duration) -> Cmd {
        self.set_timeout(Some(timeout));
        self
    }

    /// Set or clear timeout.
    pub fn set_timeout(&mut self, timeout: Option<Duration>) {
        self.deadline = timeout.map(|it| Instant::now() + it)
    }

    /// Set deadline.
    pub fn deadline(mut self, deadline: Instant) -> Cmd {
        self.set_deadline(Some(deadline));
        self
    }

    /// Set or clear deadline.
    pub fn set_deadline(&mut self, deadline: Option<Instant>) {
        self.deadline = deadline;
    }

    /// Marks the command as secret.
    ///
    /// If a command is secret, it echoes `<secret>` instead of the program and
    /// its arguments, even in error messages.
    pub fn secret(mut self) -> Cmd {
        self.set_secret(true);
        self
    }

    /// Controls whether the command is secret.
    pub fn set_secret(&mut self, yes: bool) {
        self.secret = yes;
    }

    /// Run the command for side effects without printing anything.
    ///
    /// Use this in batch scripts that don't need to report intermediate progress (for example, in
    /// tests).  If the execution fails, the error will contain a suffix of stderr and stdout for
    /// debugging.
    ///
    /// Internally, command's stdin is set to null, while stderr and stdout are piped.
    pub fn run(&self) -> Result<()> {
        let command = self.to_command();

        let mut result = exec::exec(
            command,
            self.stdin_contents.as_deref(),
            Some(STREAM_SUFFIX_SIZE),
            Some(STREAM_SUFFIX_SIZE),
            self.deadline,
        );
        self.check_exec_result(&mut result)?;
        Ok(())
    }

    fn check_exec_result(&self, result: &mut exec::ExecResult) -> Result<()> {
        if let Some(status) = result.status {
            if !status.success() && !self.ignore_status {
                return Err(Error::new_cmd(
                    self,
                    CmdErrorKind::Status(status),
                    mem::take(&mut result.stdout),
                    mem::take(&mut result.stderr),
                ));
            }
        }
        if let Some(err) = result.error.take() {
            if err.kind() == io::ErrorKind::TimedOut {
                return Err(Error::new_cmd(
                    self,
                    CmdErrorKind::Timeout,
                    mem::take(&mut result.stdout),
                    mem::take(&mut result.stderr),
                ));
            }
            return Err(Error::new_cmd(
                self,
                CmdErrorKind::Io(err),
                mem::take(&mut result.stdout),
                mem::take(&mut result.stderr),
            ));
        }
        Ok(())
    }

    /// Run the command for side effect, printing the command itself and its output.
    ///
    /// Use this in interactive scenarios (when the human looks at the command being executed in
    /// real time).
    ///
    /// Internally, command's stdin is set to null, while stderr and stdout are inherited.
    pub fn run_echo(&self) -> Result<()> {
        let mut command = self.to_command();
        command.stdin(Stdio::null());
        command.stdout(Stdio::inherit());
        command.stderr(Stdio::inherit());
        eprintln!("$ {}", self);
        let mut child = command
            .spawn()
            .map_err(|err| Error::new_cmd(self, CmdErrorKind::Io(err), Vec::new(), Vec::new()))?;
        let status = exec::wait_deadline(&mut child, self.deadline)
            .map_err(|err| Error::new_cmd(self, CmdErrorKind::Io(err), Vec::new(), Vec::new()))?;
        if !status.success() {
            return Err(Error::new_cmd(self, CmdErrorKind::Status(status), Vec::new(), Vec::new()));
        }

        Ok(())
    }

    /// Like `exec_echo`, but also inherit stdin.
    ///
    /// Use this when the user needs to type some input in.
    pub fn run_interactive(&self) -> Result<()> {
        let mut command = self.to_command();
        command.stdin(Stdio::inherit());
        command.stdout(Stdio::inherit());
        command.stderr(Stdio::inherit());
        eprintln!("$ {}", self);
        let mut child = command
            .spawn()
            .map_err(|err| Error::new_cmd(self, CmdErrorKind::Io(err), Vec::new(), Vec::new()))?;
        let status = exec::wait_deadline(&mut child, self.deadline)
            .map_err(|err| Error::new_cmd(self, CmdErrorKind::Io(err), Vec::new(), Vec::new()))?;
        if !status.success() {
            return Err(Error::new_cmd(self, CmdErrorKind::Status(status), Vec::new(), Vec::new()));
        }
        Ok(())
    }

    /// Run the command and read its standard output to string.
    ///
    /// If the output is exactly one line, the final newline is stripped.
    pub fn read(&self) -> Result<String> {
        let command = self.to_command();
        let mut result = exec::exec(
            command,
            self.stdin_contents.as_deref(),
            None,
            Some(STREAM_SUFFIX_SIZE),
            self.deadline,
        );
        self.check_exec_result(&mut result)?;
        self.chomp(result.stdout)
    }

    /// Run the command and read its standard error to string.
    ///
    /// If the output is exactly one line, the final newline is stripped.
    pub fn read_stderr(&self) -> Result<String> {
        let command = self.to_command();
        let mut result = exec::exec(
            command,
            self.stdin_contents.as_deref(),
            Some(STREAM_SUFFIX_SIZE),
            None,
            self.deadline,
        );
        self.check_exec_result(&mut result)?;
        self.chomp(result.stderr)
    }

    fn chomp(&self, stream: Vec<u8>) -> Result<String> {
        let mut text = String::from_utf8(stream)
            .map_err(|err| Error::new_cmd(self, CmdErrorKind::Utf8(err), Vec::new(), Vec::new()))?;
        if text.ends_with('\n') && !text[0..text.len() - 1].contains('\n') {
            text.pop();
            if text.ends_with('\r') {
                text.pop();
            }
        }
        Ok(text)
    }

    /// Run the command and return its full output.
    pub fn output(&self) -> Result<Output> {
        let mut command = self.to_command();
        command.stdin(Stdio::null());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        let mut result =
            exec::exec(command, self.stdin_contents.as_deref(), None, None, self.deadline);
        self.check_exec_result(&mut result)?;
        Ok(Output {
            status: result.status.take().unwrap(),
            stdout: result.stdout,
            stderr: result.stderr,
        })
    }

    /// Constructs a [`std::process::Command`] for the same command as `self`.
    ///
    /// The returned command will invoke the same program from the same working directory and with
    /// the same environment as `self`.  If the command was set to
    /// [`ignore_stdout`](Cmd::ignore_stdout) or [`ignore_stderr`](Cmd::ignore_stderr), this will
    /// apply to the returned command as well.
    ///
    /// Other builder methods have no effect on the command returned since they control how the
    /// command is run, but this method does not yet execute the command.
    pub fn to_command(&self) -> Command {
        let mut result = Command::new(&self.prog);
        result.current_dir(&self.sh.cwd);
        result.args(&self.args);
        result.envs(&*self.sh.env);
        result
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
