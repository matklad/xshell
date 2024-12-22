use std::{
    env,
    ffi::OsString,
    fmt, io,
    path::{Path, PathBuf},
    process::ExitStatus,
    string::FromUtf8Error,
    sync::Arc,
};

use libc::SOCK_DGRAM;

use crate::Cmd;

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
    CmdStatus { cmd: Cmd, status: ExitStatus },
    CmdIo { err: io::Error, cmd: Cmd },
    CmdUtf8 { err: FromUtf8Error, cmd: Cmd },
    CmdStdin { err: io::Error, cmd: Cmd },
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
            ErrorKind::Cmd(cmd) => fmt::Display::fmt(f, cmd),

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
                    let prog = cmd.prog.as_path().display();
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

impl fmt::Display for CmdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let nl = if (self.stdout.len() > 0 || self.stderr.len() > 0)
            && !matches!(self.kind, CmdErrorKind::Utf8(_))
        {
            "\n"
        } else {
            ""
        };
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
                write!(f, "command failed `{cmd}`: {err}{nl}")?;
            }
            CmdErrorKind::Timeout => {
                write!(f, "command timed out `{cmd}`{nl}")?;
            }
        }
        if (self.stdout.len() > 0) {
            write!(f, "stdout suffix\n:{}\n", String::from_utf8_lossy(&self.stdout))?;
        }
        if (self.stderr.len() > 0) {
            write!(f, "stderr suffix:\n:{}\n", String::from_utf8_lossy(&self.stderr))?;
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
        stdout: Vec<u8>,
        stderr: Vec<u8>,
    ) -> Error {
        let cmd = cmd.clone();
        ErrorKind::Cmd(CmdError { cmd, kind, stdout, stderr }).into()
    }

    pub(crate) fn new_cmd_io(cmd: &Cmd, err: io::Error) -> Error {
        let cmd = cmd.clone();
        ErrorKind::CmdIo { err, cmd }.into()
    }

    pub(crate) fn new_cmd_utf8(cmd: &Cmd, err: FromUtf8Error) -> Error {
        let cmd = cmd.clone();
        ErrorKind::CmdUtf8 { err, cmd }.into()
    }

    pub(crate) fn new_cmd_stdin(cmd: &Cmd, err: io::Error) -> Error {
        let cmd = cmd.clone();
        ErrorKind::CmdStdin { err, cmd }.into()
    }
}

#[test]
fn error_send_sync() {
    fn f<T: Send + Sync>() {}
    f::<Error>();
}
