use std::{
    collections::VecDeque,
    io::{self, Write},
    process::{Child, ChildStdin, ExitStatus, Stdio},
    time::Instant,
};

#[derive(Default)]
pub(crate) struct Exec3Result {
    pub(crate) stdout: Vec<u8>,
    pub(crate) stderr: Vec<u8>,
    pub(crate) status: Option<ExitStatus>,
    pub(crate) error: Option<io::Error>,
}

pub(crate) fn exec3(
    mut command: std::process::Command,
    stdin_contents: Option<&[u8]>,
    stdout_limit: Option<usize>,
    stderr_limit: Option<usize>,
    deadline: Option<Instant>,
) -> Exec3Result {
    let mut result = Exec3Result::default();
    command.stdin(if (stdin_contents.is_some()) { Stdio::inherit() } else { Stdio::null() });
    command.stdout(Stdio::piped());
    command.stdout(Stdio::piped());
    let mut child = match command.spawn() {
        Ok(it) => it,
        Err(err) => {
            result.error = Some(err);
            return result;
        }
    };

    let mut out_deque = VecDeque::new();
    let mut err_deque = VecDeque::new();
    let mut in_written: usize = 0;

    result.error = imp::read3(
        child.stdin,
        child.stdout.unwrap(),
        child.stderr.unwrap(),
        deadline,
        &mut |event| match event {
            Event::Read { stdout, data } => {
                let (deque, limit) = if stdout {
                    (&mut out_deque, stdout_limit)
                } else {
                    (&mut err_deque, stderr_limit)
                };
                deque.extend(data.iter().copied());
                if let Some(limit) = limit {
                    let excess = deque.len().saturating_sub(limit);
                    if excess > 0 {
                        deque.drain(0..excess);
                    }
                }
                Ok(())
            }
            Event::Write { child } => {
                let stdin_contents = stdin_contents.unwrap();
                let n = child.as_mut().unwrap().write(&stdin_contents[in_written..])?;
                in_written += n;
                if in_written == stdin_contents.len() {
                    *child = None;
                }
                Ok(())
            }
        },
    )
    .err();

    child.try_wait()
    match child.wait() {
        Ok(status) => result.status = Some(status),
        Err(err) => {
            if result.error.is_none() {
                result.error = Some(err);
            }
        }
    }

    result
}

enum Event {
    Read { stdout: bool, data: &[u8] },
    Write { child: &mut Option<ChildStdin> },
}

#[cfg(unix)]
mod imp {
    use libc::{c_int, fcntl, F_GETFL, F_SETFL, O_NONBLOCK};
    use std::io;
    use std::io::prelude::*;
    use std::mem;
    use std::os::unix::prelude::*;
    use std::process::{ChildStderr, ChildStdin, ChildStdout};
    use std::time::{Duration, Instant};

    use super::Event;

    fn set_nonblock(fd: c_int) -> io::Result<()> {
        let flags = unsafe { fcntl(fd, F_GETFL) };
        if flags == -1 || unsafe { fcntl(fd, F_SETFL, flags | O_NONBLOCK) } == -1 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    pub fn read3(
        mut in_pipe: Option<ChildStdin>,
        mut out_pipe: ChildStdout,
        mut err_pipe: ChildStderr,
        deadline: Option<Instant>,
        data: &mut dyn FnMut(Event) -> io::Result<()>,
    ) -> io::Result<()> {
        if let Some(in_pipe) = &in_pipe {
            set_nonblock(in_pipe.as_raw_fd())?;
        }
        set_nonblock(out_pipe.as_raw_fd())?;
        set_nonblock(err_pipe.as_raw_fd())?;

        let mut fds: [libc::pollfd; 3] = unsafe { mem::zeroed() };
        let mut fds_count: usize = 0;

        let mut out_done = false;
        let mut out_fd = fds_count;
        fds[fds_count as usize].fd = out_pipe.as_raw_fd();
        fds[fds_count as usize].events = libc::POLLIN;
        fds_count += 1;

        let mut err_done = false;
        let mut err_fd = fds_count;
        fds[fds_count as usize].fd = err_pipe.as_raw_fd();
        fds[fds_count as usize].events = libc::POLLIN;
        fds_count += 1;

        let mut in_done = true;
        let mut in_fd = fds_count;
        if let Some(in_pipe) = &in_pipe {
            in_done = false;
            fds[fds_count as usize].fd = in_pipe.as_raw_fd();
            fds[fds_count as usize].events = libc::POLLOUT;
            fds_count += 1;
        }

        let mut buffer = [0; 4096];
        while fds_count > 0 {
            let timeout_ms: c_int = match deadline {
                Some(deadline) => {
                    let timeout = deadline
                        .checked_duration_since(Instant::now())
                        .ok_or_else(|| io::ErrorKind::TimedOut.into())?;
                    timeout.as_millis().clamp(0, c_int::MAX as u128) as c_int
                }
                None => -1,
            };
            let r = unsafe { libc::poll(fds.as_mut_ptr(), fds_count as u32, timeout_ms) };
            if r == 0 {
                return Err(io::ErrorKind::TimedOut.into());
            }
            if r == -1 {
                let err = io::Error::last_os_error();
                if err.kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(err);
            }

            if !in_done && fds[in_fd].revents != 0 {
                match data(Event::Write { child: &mut in_pipe }) {
                    Ok(()) => {}
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
                    Err(e) if e.kind() == io::ErrorKind::BrokenPipe => {
                        *in_pipe = None;
                    }
                    Err(e) => return Err(e),
                }
                if in_pipe.is_none() {
                    in_done = true;
                    fds_count -= 1;
                }
            }

            if !err_done && fds[err_fd].revents != 0 {
                match err_pipe.read(&mut buffer) {
                    Ok(n) => {
                        if (n == 0) {
                            err_done = true;
                            in_fd -= 1;
                            fds_count -= 1;
                        } else {
                            data(Event::Read { stdout: false, data: &buffer[..n] })?;
                        }
                    }
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
                    Err(e) => return Err(e),
                }
            }

            if !out_done && fds[0].revents != 0 {
                match out_pipe.read(&mut buffer) {
                    Ok(n) => {
                        if (n == 0) {
                            out_done = true;
                            in_fd -= 1;
                            err_fd -= 1;
                            fds_count -= 1;
                        } else {
                            data(Event::Read { stdout: true, data: &buffer[..n] })?;
                        }
                    }
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
                    Err(e) => return Err(e),
                }
            }
        }
        Ok(())
    }
}

#[cfg(windows)]
mod imp {
    use std::io;
    use std::os::windows::prelude::*;
    use std::process::{ChildStderr, ChildStdout};
    use std::slice;

    use miow::iocp::{CompletionPort, CompletionStatus};
    use miow::pipe::NamedPipe;
    use miow::Overlapped;
    use windows_sys::Win32::Foundation::ERROR_BROKEN_PIPE;

    struct Pipe<'a> {
        dst: &'a mut Vec<u8>,
        overlapped: Overlapped,
        pipe: NamedPipe,
        done: bool,
    }

    pub fn read2(
        out_pipe: ChildStdout,
        err_pipe: ChildStderr,
        data: &mut dyn FnMut(bool, &mut Vec<u8>, bool),
    ) -> io::Result<()> {
        let mut out = Vec::new();
        let mut err = Vec::new();

        let port = CompletionPort::new(1)?;
        port.add_handle(0, &out_pipe)?;
        port.add_handle(1, &err_pipe)?;

        unsafe {
            let mut out_pipe = Pipe::new(out_pipe, &mut out);
            let mut err_pipe = Pipe::new(err_pipe, &mut err);

            out_pipe.read()?;
            err_pipe.read()?;

            let mut status = [CompletionStatus::zero(), CompletionStatus::zero()];

            while !out_pipe.done || !err_pipe.done {
                for status in port.get_many(&mut status, None)? {
                    if status.token() == 0 {
                        out_pipe.complete(status);
                        data(true, out_pipe.dst, out_pipe.done);
                        out_pipe.read()?;
                    } else {
                        err_pipe.complete(status);
                        data(false, err_pipe.dst, err_pipe.done);
                        err_pipe.read()?;
                    }
                }
            }

            Ok(())
        }
    }

    impl<'a> Pipe<'a> {
        unsafe fn new<P: IntoRawHandle>(p: P, dst: &'a mut Vec<u8>) -> Pipe<'a> {
            Pipe {
                dst,
                pipe: NamedPipe::from_raw_handle(p.into_raw_handle()),
                overlapped: Overlapped::zero(),
                done: false,
            }
        }

        unsafe fn read(&mut self) -> io::Result<()> {
            let dst = slice_to_end(self.dst);
            match self.pipe.read_overlapped(dst, self.overlapped.raw()) {
                Ok(_) => Ok(()),
                Err(e) => {
                    if e.raw_os_error() == Some(ERROR_BROKEN_PIPE as i32) {
                        self.done = true;
                        Ok(())
                    } else {
                        Err(e)
                    }
                }
            }
        }

        unsafe fn complete(&mut self, status: &CompletionStatus) {
            let prev = self.dst.len();
            self.dst.set_len(prev + status.bytes_transferred() as usize);
            if status.bytes_transferred() == 0 {
                self.done = true;
            }
        }
    }

    unsafe fn slice_to_end(v: &mut Vec<u8>) -> &mut [u8] {
        if v.capacity() == 0 {
            v.reserve(16);
        }
        if v.capacity() == v.len() {
            v.reserve(1);
        }
        slice::from_raw_parts_mut(v.as_mut_ptr().add(v.len()), v.capacity() - v.len())
    }
}
