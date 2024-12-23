use std::{
    env,
    ffi::OsString,
    fmt, io,
    path::{Path, PathBuf},
    process::ExitStatus,
    string::FromUtf8Error,
    sync::Arc,
};

use crate::{Cmd, STREAM_SUFFIX_SIZE};

/// `Result` from std, with the error type defaulting to xshell's [`Error`].
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// An error returned by an `xshell` operation.
pub struct Error {
    kind: Box<ErrorKind>,
}

/// Note: this is intentionally not public.
enum ErrorKind {
    CurrentDir { err: io::Error, path: Option<Arc<Path>> },
    Var { err: env::VarError, var: OsString },
    ReadFile { err: io::Error, path: PathBuf },
    ReadDir { err: io::Error, path: PathBuf },
    WriteFile { err: io::Error, path: PathBuf },
    CopyFile { err: io::Error, src: PathBuf, dst: PathBuf },
    HardLink { err: io::Error, src: PathBuf, dst: PathBuf },
    CreateDir { err: io::Error, path: PathBuf },
    RemovePath { err: io::Error, path: PathBuf },
    Cmd(CmdError),
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Error {
        let kind = Box::new(kind);
        Error { kind }
    }
}

struct CmdError {
    cmd: Cmd,
    kind: CmdErrorKind,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

pub(crate) enum CmdErrorKind {
    Io(io::Error),
    Utf8(FromUtf8Error),
    Status(ExitStatus),
    Timeout,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &*self.kind {
            ErrorKind::CurrentDir { err, path } => {
                let suffix =
                    path.as_ref().map_or(String::new(), |path| format!(" `{}`", path.display()));
                write!(f, "failed to get current directory{suffix}: {err}")
            }
            ErrorKind::Var { err, var } => {
                let var = var.to_string_lossy();
                write!(f, "failed to get environment variable `{var}`: {err}")
            }
            ErrorKind::ReadFile { err, path } => {
                let path = path.display();
                write!(f, "failed to read file `{path}`: {err}")
            }
            ErrorKind::ReadDir { err, path } => {
                let path = path.display();
                write!(f, "failed read directory `{path}`: {err}")
            }
            ErrorKind::WriteFile { err, path } => {
                let path = path.display();
                write!(f, "failed to write file `{path}`: {err}")
            }
            ErrorKind::CopyFile { err, src, dst } => {
                let src = src.display();
                let dst = dst.display();
                write!(f, "failed to copy `{src}` to `{dst}`: {err}")
            }
            ErrorKind::HardLink { err, src, dst } => {
                let src = src.display();
                let dst = dst.display();
                write!(f, "failed hard link `{src}` to `{dst}`: {err}")
            }
            ErrorKind::CreateDir { err, path } => {
                let path = path.display();
                write!(f, "failed to create directory `{path}`: {err}")
            }
            ErrorKind::RemovePath { err, path } => {
                let path = path.display();
                write!(f, "failed to remove path `{path}`: {err}")
            }
            ErrorKind::Cmd(cmd) => fmt::Display::fmt(cmd, f),
        }?;
        Ok(())
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl std::error::Error for Error {}

impl fmt::Display for CmdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let nl = if (self.stdout.len() > 0 || self.stderr.len() > 0)
            && !matches!(self.kind, CmdErrorKind::Utf8(_))
        {
            "\n"
        } else {
            ""
        };
        let cmd = &self.cmd;
        match &self.kind {
            CmdErrorKind::Status(status) => match status.code() {
                Some(code) => write!(f, "command exited with non-zero code `{cmd}`: {code}{nl}")?,
                #[cfg(unix)]
                None => {
                    use std::os::unix::process::ExitStatusExt;
                    match status.signal() {
                        Some(sig) => {
                            write!(f, "command was terminated by a signal `{cmd}`: {sig}{nl}")?
                        }
                        None => write!(f, "command was terminated by a signal `{cmd}`{nl}")?,
                    }
                }
                #[cfg(not(unix))]
                None => write!(f, "command was terminated by a signal `{cmd}`{nl}"),
            },
            CmdErrorKind::Utf8(err) => {
                write!(f, "command produced invalid utf-8 `{cmd}`: {err}")?;
                return Ok(());
            }
            CmdErrorKind::Io(err) => {
                if err.kind() == io::ErrorKind::NotFound {
                    let prog = self.cmd.prog.as_path().display();
                    write!(f, "command not found: `{prog}`{nl}")?;
                } else {
                    write!(f, "io error when running command `{cmd}`: {err}{nl}")?;
                }
            }
            CmdErrorKind::Timeout => {
                write!(f, "command timed out `{cmd}`{nl}")?;
            }
        }
        if self.stdout.len() > 0 {
            write!(f, "stdout suffix\n{}\n", String::from_utf8_lossy(&self.stdout))?;
        }
        if self.stderr.len() > 0 {
            write!(f, "stderr suffix:\n{}\n", String::from_utf8_lossy(&self.stderr))?;
        }
        Ok(())
    }
}

/// `pub(crate)` constructors, visible only in this crate.
impl Error {
    pub(crate) fn new_current_dir(err: io::Error, path: Option<Arc<Path>>) -> Error {
        ErrorKind::CurrentDir { err, path }.into()
    }

    pub(crate) fn new_var(err: env::VarError, var: OsString) -> Error {
        ErrorKind::Var { err, var }.into()
    }

    pub(crate) fn new_read_file(err: io::Error, path: PathBuf) -> Error {
        ErrorKind::ReadFile { err, path }.into()
    }

    pub(crate) fn new_read_dir(err: io::Error, path: PathBuf) -> Error {
        ErrorKind::ReadDir { err, path }.into()
    }

    pub(crate) fn new_write_file(err: io::Error, path: PathBuf) -> Error {
        ErrorKind::WriteFile { err, path }.into()
    }

    pub(crate) fn new_copy_file(err: io::Error, src: PathBuf, dst: PathBuf) -> Error {
        ErrorKind::CopyFile { err, src, dst }.into()
    }

    pub(crate) fn new_hard_link(err: io::Error, src: PathBuf, dst: PathBuf) -> Error {
        ErrorKind::HardLink { err, src, dst }.into()
    }

    pub(crate) fn new_create_dir(err: io::Error, path: PathBuf) -> Error {
        ErrorKind::CreateDir { err, path }.into()
    }

    pub(crate) fn new_remove_path(err: io::Error, path: PathBuf) -> Error {
        ErrorKind::RemovePath { err, path }.into()
    }

    pub(crate) fn new_cmd(
        cmd: &Cmd,
        kind: CmdErrorKind,
        mut stdout: Vec<u8>,
        mut stderr: Vec<u8>,
    ) -> Error {
        // Try to determine whether the command failed because the current
        // directory does not exist. Return an appropriate error in such a
        // case.
        if let CmdErrorKind::Io(err) = &kind {
            if err.kind() == io::ErrorKind::NotFound {
                if let Err(err) = cmd.sh.cwd.metadata() {
                    return Error::new_current_dir(err, Some(cmd.sh.cwd.clone()));
                }
            }
        }

        fn trim(xs: &mut Vec<u8>, size: usize) {
            if xs.len() > size {
                xs.drain(..xs.len() - size);
            }
        }

        let cmd = cmd.clone();
        trim(&mut stdout, STREAM_SUFFIX_SIZE);
        trim(&mut stderr, STREAM_SUFFIX_SIZE);
        ErrorKind::Cmd(CmdError { cmd, kind, stdout, stderr }).into()
    }
}

#[test]
fn error_send_sync() {
    fn f<T: Send + Sync>() {}
    f::<Error>();
}
