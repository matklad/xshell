use std::{fmt, io, path::PathBuf, process::ExitStatus, string::FromUtf8Error};

use crate::Cmd;

/// `Result` from std, with the error type defaulting to xshell's [`Error`].
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// An error returned by an `xshell` operation.
pub struct Error {
    repr: Box<Repr>,
}

enum Repr {
    CmdError(CmdError),
    FsError(FsError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn errstr(err: &io::Error) -> String {
            let mut res = err.to_string();
            if res.is_char_boundary(1) {
                res[..1].make_ascii_lowercase();
            }
            res
        }
        match &*self.repr {
            Repr::CmdError(err) => match &err.kind {
                CmdErrorKind::NonZeroStatus(status) => {
                    write!(f, "command `{}` failed, {}", err.cmd, status)
                }
                CmdErrorKind::Io(io_err) => {
                    if io_err.kind() == io::ErrorKind::NotFound {
                        write!(f, "command not found: `{}`", err.cmd.args[0].to_string_lossy())
                    } else {
                        write!(f, "command `{}` failed, {}", err.cmd, errstr(io_err))
                    }
                }
                CmdErrorKind::NonUtf8Output(utf8_err) => {
                    write!(f, "command `{}` produced invalid utf8, {}", err.cmd, utf8_err)
                }
            },
            Repr::FsError(err) => write!(f, "`{}`: {}", err.path.display(), errstr(&err.io_err)),
        }
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl std::error::Error for Error {}

pub(crate) struct CmdError {
    cmd: Cmd,
    kind: CmdErrorKind,
}

pub(crate) enum CmdErrorKind {
    NonZeroStatus(ExitStatus),
    Io(io::Error),
    NonUtf8Output(FromUtf8Error),
}

impl CmdErrorKind {
    pub(crate) fn err(self, cmd: Cmd) -> Error {
        Error { repr: Box::new(Repr::CmdError(CmdError { cmd, kind: self })) }
    }
}

pub(crate) struct FsError {
    path: PathBuf,
    io_err: io::Error,
}

pub(crate) fn fs_err(path: PathBuf, io_err: io::Error) -> Error {
    Error { repr: Box::new(Repr::FsError(FsError { path, io_err })) }
}
