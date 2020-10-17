//! xshell, making Rust a better bash.
//!
//! Docs are incoming, take a look at the `examples/ci.rs` in the meantime.

mod env;
mod gsl;
mod error;
mod fs;

use std::{
    ffi::{OsStr, OsString},
    fmt, io,
    io::Write,
    path::Path,
    process::Output,
    process::Stdio,
};

use error::CmdErrorKind;
#[doc(hidden)]
pub use xshell_macros::__cmd;

pub use crate::{
    env::{pushd, pushenv, Pushd, Pushenv},
    error::{Error, Result},
    fs::{cp, cwd, mkdir_p, read_dir, read_file, rm_rf, write_file},
};

#[macro_export]
macro_rules! cmd {
    ($cmd:tt) => {{
        #[cfg(trick_rust_analyzer_into_highlighting_interpolated_bits)]
        format_args!($cmd);
        use $crate::Cmd as __CMD;
        let cmd: $crate::Cmd = $crate::__cmd!(__CMD $cmd);
        cmd
    }};
}

#[must_use]
#[derive(Debug)]
pub struct Cmd {
    args: Vec<OsString>,
    stdin_contents: Option<Vec<u8>>,
}

impl fmt::Display for Cmd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut space = "";
        for arg in &self.args {
            write!(f, "{}", space)?;
            space = " ";

            let arg = arg.to_string_lossy();
            if arg.chars().any(|it| it.is_ascii_whitespace()) {
                write!(f, "\"{}\"", arg.escape_default())?
            } else {
                write!(f, "{}", arg)?
            };
        }
        Ok(())
    }
}

impl From<Cmd> for std::process::Command {
    fn from(cmd: Cmd) -> Self {
        cmd.command()
    }
}

impl Cmd {
    pub fn new(program: impl AsRef<Path>) -> Cmd {
        Cmd::_new(program.as_ref())
    }
    fn _new(program: &Path) -> Cmd {
        Cmd { args: vec![program.as_os_str().to_owned()], stdin_contents: None }
    }

    pub fn arg(mut self, arg: impl AsRef<OsStr>) -> Cmd {
        self._arg(arg.as_ref());
        self
    }
    pub fn args<I>(mut self, args: I) -> Cmd
    where
        I: IntoIterator,
        I::Item: AsRef<OsStr>,
    {
        args.into_iter().for_each(|it| self._arg(it.as_ref()));
        self
    }
    pub fn arg_if(mut self, cond: bool, arg: impl AsRef<OsStr>) -> Cmd {
        if cond {
            self._arg(arg.as_ref())
        }
        self
    }

    fn _arg(&mut self, arg: &OsStr) {
        self.args.push(arg.to_owned())
    }

    #[doc(hidden)]
    pub fn __extend_arg(mut self, arg: impl AsRef<OsStr>) -> Cmd {
        self.___extend_arg(arg.as_ref());
        self
    }
    fn ___extend_arg(&mut self, arg: &OsStr) {
        self.args.last_mut().unwrap().push(arg)
    }

    pub fn stdin(mut self, stdin: impl AsRef<[u8]>) -> Cmd {
        self._stdin(stdin.as_ref());
        self
    }
    fn _stdin(&mut self, stdin: &[u8]) {
        self.stdin_contents = Some(stdin.to_vec());
    }

    pub fn read(self) -> Result<String> {
        match self.read_raw() {
            Ok(output) if output.status.success() => {
                let mut stdout = String::from_utf8(output.stdout)
                    .map_err(|utf8_err| CmdErrorKind::NonUtf8Stdout(utf8_err).err(self))?;
                if stdout.ends_with('\n') {
                    stdout.pop();
                }

                Ok(stdout)
            }
            Ok(output) => Err(CmdErrorKind::NonZeroStatus(output.status).err(self)),
            Err(io_err) => Err(CmdErrorKind::Io(io_err).err(self)),
        }
    }
    fn read_raw(&self) -> io::Result<Output> {
        let mut child = self
            .command()
            .stdin(match &self.stdin_contents {
                Some(_) => Stdio::piped(),
                None => Stdio::null(),
            })
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        if let Some(stdin_contents) = &self.stdin_contents {
            let mut stdin = child.stdin.take().unwrap();
            stdin.write_all(stdin_contents)?;
            stdin.flush()?;
        }
        child.wait_with_output()
    }

    pub fn run(self) -> Result<()> {
        println!("$ {}", self);
        match self.command().status() {
            Ok(status) if status.success() => Ok(()),
            Ok(status) => Err(CmdErrorKind::NonZeroStatus(status).err(self)),
            Err(io_err) => Err(CmdErrorKind::Io(io_err).err(self)),
        }
    }

    fn command(&self) -> std::process::Command {
        let mut res = std::process::Command::new(&self.args[0]);
        res.args(&self.args[1..]);
        res
    }
}
