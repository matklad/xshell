use std::{env, ffi::OsString, fmt, io, path::PathBuf, process::ExitStatus, string::FromUtf8Error};

use crate::{Cmd, CmdData};

/// `Result` from std, with the error type defaulting to xshell's [`Error`].
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// An error returned by an `xshell` operation.
pub struct Error {
    kind: Box<ErrorKind>,
}

/// Note: this is intentionally not public.
enum ErrorKind {
    CurrentDir { err: io::Error },
    Var { err: env::VarError, var: OsString },
    ReadFile { err: io::Error, path: PathBuf },
    ReadDir { err: io::Error, path: PathBuf },
    WriteFile { err: io::Error, path: PathBuf },
    CopyFile { err: io::Error, src: PathBuf, dst: PathBuf },
    HardLink { err: io::Error, src: PathBuf, dst: PathBuf },
    CreateDir { err: io::Error, path: PathBuf },
    RemovePath { err: io::Error, path: PathBuf },
    CmdStatus { cmd: CmdData, status: ExitStatus },
    CmdIo { err: io::Error, cmd: CmdData },
    CmdUtf8 { err: FromUtf8Error, cmd: CmdData },
    CmdStdin { err: io::Error, cmd: CmdData },
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Error {
        let kind = Box::new(kind);
        Error { kind }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &*self.kind {
            ErrorKind::CurrentDir { err } => write!(f, "failed to get current directory: {err}"),
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
            ErrorKind::CmdStatus { cmd, status } => match status.code() {
                Some(code) => write!(f, "command exited with non-zero code `{cmd}`: {code}"),
                #[cfg(unix)]
                None => {
                    use std::os::unix::process::ExitStatusExt;
                    match status.signal() {
                        Some(sig) => write!(f, "command was terminated by a signal `{cmd}`: {sig}"),
                        None => write!(f, "command was terminated by a signal `{cmd}`"),
                    }
                }
                #[cfg(not(unix))]
                None => write!(f, "command was terminated by a signal `{cmd}`"),
            },
            ErrorKind::CmdIo { err, cmd } => {
                if err.kind() == io::ErrorKind::NotFound {
                    let prog = cmd.prog.display();
                    write!(f, "command not found: `{prog}`")
                } else {
                    write!(f, "io error when running command `{cmd}`: {err}")
                }
            }
            ErrorKind::CmdUtf8 { err, cmd } => {
                write!(f, "failed to decode output of command `{cmd}`: {err}")
            }
            ErrorKind::CmdStdin { err, cmd } => {
                write!(f, "failed to write to stdin of command `{cmd}`: {err}")
            }
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

/// `pub(crate)` constructors, visible only in this crate.
impl Error {
    pub(crate) fn new_current_dir(err: io::Error) -> Error {
        ErrorKind::CurrentDir { err }.into()
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

    pub(crate) fn new_cmd_status(cmd: &Cmd<'_>, status: ExitStatus) -> Error {
        let cmd = cmd.data.clone();
        ErrorKind::CmdStatus { cmd, status }.into()
    }

    pub(crate) fn new_cmd_io(cmd: &Cmd<'_>, err: io::Error) -> Error {
        let cmd = cmd.data.clone();
        ErrorKind::CmdIo { err, cmd }.into()
    }

    pub(crate) fn new_cmd_utf8(cmd: &Cmd<'_>, err: FromUtf8Error) -> Error {
        let cmd = cmd.data.clone();
        ErrorKind::CmdUtf8 { err, cmd }.into()
    }

    pub(crate) fn new_cmd_stdin(cmd: &Cmd<'_>, err: io::Error) -> Error {
        let cmd = cmd.data.clone();
        ErrorKind::CmdStdin { err, cmd }.into()
    }
}

#[test]
fn error_send_sync() {
    fn f<T: Send + Sync>() {}
    f::<Error>();
}
