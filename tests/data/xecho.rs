use std::io::{self, Write};

fn main() {
    if let Err(err) = try_main() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn try_main() -> io::Result<()> {
    let mut tee_stderr = false;
    let mut echo_stdin = false;
    let mut echo_env = false;
    let mut fail = false;
    let mut suicide = false;

    let mut args = std::env::args().skip(1).peekable();
    while let Some(arg) = args.peek() {
        match arg.as_str() {
            "-e" => tee_stderr = true,
            "-i" => echo_stdin = true,
            "-$" => echo_env = true,
            "-f" => fail = true,
            "-s" => suicide = true,
            _ => break,
        }
        args.next();
    }

    let stdin = io::stdin();
    let stdout = io::stdout();
    let stderr = io::stderr();
    let mut stdin = stdin.lock();
    let mut stdout = stdout.lock();
    let mut stderr = stderr.lock();
    macro_rules! w {
        ($($tt:tt)*) => {
            write!(stdout, $($tt)*)?;
            if tee_stderr {
                write!(stderr, $($tt)*)?;
            }
        }
    }

    if echo_stdin {
        io::copy(&mut stdin, &mut stdout)?;
    } else if echo_env {
        for key in args {
            if let Some(v) = std::env::var_os(&key) {
                w!("{}={}\n", key, v.to_string_lossy());
            }
        }
    } else {
        let mut space = "";
        for arg in args {
            w!("{}{}", space, arg);
            space = " ";
        }
        w!("\n");
    }

    if fail {
        return Err(io::ErrorKind::Other.into());
    }
    if suicide {
        unsafe {
            let pid = getpid();
            if pid > 0 {
                kill(pid, 9);
            }
        }
    }

    Ok(())
}

use std::os::raw::c_int;
extern "C" {
    fn kill(pid: c_int, sig: c_int) -> c_int;
    fn getpid() -> c_int;
}
