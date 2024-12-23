//! Executes the process, feeding it stdin, reading stdout/stderr (up to the specified limit), and
//! imposing a deadline.
//!
//! This really is quite unhappy code, wasting whopping four threads for the task _and_ including a
//! sleepy loop! This is not system programming, just a pile of work-around. What is my excuse?
//!
//! The _right_ way to do this is of course by using evened syscalls --- concurrently await stream
//! io, timeout, and process termination. The _first_ two kinda-sorta solvable, see the `read2`
//! module in Cargo. For unix, we through fds into a epoll via libc, for windows we use completion
//! ports via miow. That's some ugly platform-specific code and two dependencies, but doable.
//!
//! Both poll and completion ports naturally have a timeout, so that's doable as well. However,
//! tying process termination into the same epoll is not really possible. One can use pidfd's on
//! Linux, but that's even _more_ platform specific code, and there are other UNIXes.
//!
//! Given that, if I were to use evented IO, I'd have to pull dependencies, write a bunch of
//! platform-specific glue code _and_ write some from scratch things for waiting, I decided to stick
//! to blocking APIs.
//!
//! This should be easy, right? Just burn a thread per asynchronous operation! Well, the `wait`
//! strikes again! Both `.kill` and `.wait` require `&mut Child`, so you can't wait on the main
//! thread, and `.kill` from the timeout thread. One can think that that's just deficiency of Rust
//! API, but, now, this is again just UNIX. Both kill and wait operate on pids, and a pid can be
//! re-used immediately after wait. As far as I understand, this is a race condition you can't lock
//! your way out of. Hence the sleepy loop in wait_deadline.

use std::{
    collections::VecDeque,
    io::{self, Read, Write},
    process::{Child, ExitStatus, Stdio},
    time::{Duration, Instant},
};

#[derive(Default)]
pub(crate) struct ExecResult {
    pub(crate) stdout: Vec<u8>,
    pub(crate) stderr: Vec<u8>,
    pub(crate) status: Option<ExitStatus>,
    pub(crate) error: Option<io::Error>,
}

pub(crate) fn wait_deadline(
    child: &mut Child,
    deadline: Option<Instant>,
) -> io::Result<ExitStatus> {
    let Some(deadline) = deadline else {
        return child.wait();
    };

    let mut sleep_ms = 1;
    let sleep_ms_max = 64;
    loop {
        match child.try_wait()? {
            Some(status) => return Ok(status),
            None => {}
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(io::ErrorKind::TimedOut.into());
        }
        std::thread::sleep(Duration::from_millis(sleep_ms));
        sleep_ms = std::cmp::min(sleep_ms * 2, sleep_ms_max);
    }
}

pub(crate) fn exec(
    mut command: std::process::Command,
    stdin_contents: Option<&[u8]>,
    stdout_limit: Option<usize>,
    stderr_limit: Option<usize>,
    deadline: Option<Instant>,
) -> ExecResult {
    let mut result = ExecResult::default();
    command.stdin(if stdin_contents.is_some() { Stdio::piped() } else { Stdio::null() });
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());
    let mut child = match command.spawn() {
        Ok(it) => it,
        Err(err) => {
            result.error = Some(err);
            return result;
        }
    };

    let stdin = child.stdin.take();
    let mut in_error = Ok(());

    let mut stdout = child.stdout.take().unwrap();
    let mut out_deque = VecDeque::new();
    let mut out_error = Ok(());

    let mut stderr = child.stderr.take().unwrap();
    let mut err_deque = VecDeque::new();
    let mut err_error = Ok(());

    let status = std::thread::scope(|scope| {
        if let Some(stdin_contents) = stdin_contents {
            scope.spawn(|| in_error = stdin.unwrap().write_all(stdin_contents));
        }
        scope.spawn(|| {
            out_error = (|| {
                let mut buffer = [0u8; 4096];
                loop {
                    let n = stdout.read(&mut buffer)?;
                    if n == 0 {
                        return Ok(());
                    }
                    out_deque.extend(buffer[0..n].iter().copied());
                    let excess = out_deque.len().saturating_sub(stdout_limit.unwrap_or(usize::MAX));
                    if excess > 0 {
                        out_deque.drain(..excess);
                    }
                }
            })()
        });
        scope.spawn(|| {
            err_error = (|| {
                let mut buffer = [0u8; 4096];
                loop {
                    let n = stderr.read(&mut buffer)?;
                    if n == 0 {
                        return Ok(());
                    }
                    err_deque.extend(buffer[0..n].iter().copied());
                    let excess = err_deque.len().saturating_sub(stderr_limit.unwrap_or(usize::MAX));
                    if excess > 0 {
                        err_deque.drain(..excess);
                    }
                }
            })()
        });

        wait_deadline(&mut child, deadline)
    });

    if let Err(err) = err_error {
        result.error = err;
    }

    if let Err(err) = out_error {
        result.error = err;
    }

    if let Err(err) = in_error {
        if err.kind() != io::ErrorKind::BrokenPipe {
            result.error = Some(err);
        }
    }

    match status {
        Ok(status) => result.status = Some(status),
        Err(err) => result.error = Some(err),
    }

    result.stdout = out_deque.into();
    result.stderr = err_deque.into();

    result
}
