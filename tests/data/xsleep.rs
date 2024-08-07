use std::io::{self, Write};
use std::thread;
use std::time::Duration;

fn main() {
    if let Err(err) = try_main() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn try_main() -> io::Result<()> {
    let mut sleep_seconds = 0;
    let mut fail = false;
    let mut suicide = false;

    let mut args = std::env::args().skip(1).peekable();
    while let Some(arg) = args.peek() {
        match arg.as_str() {
            "-f" => fail = true,
            "-s" => suicide = true,
            _ => break,
        }
        args.next();
    }

    if let Some(arg) = args.next() {
        sleep_seconds = arg.parse().unwrap_or_else(|_| {
            eprintln!("error: invalid number of seconds");
            std::process::exit(1);
        });
    }

    thread::sleep(Duration::from_secs(sleep_seconds));

    if fail {
        return Err(io::ErrorKind::Other.into());
    }
    if suicide {
        #[cfg(unix)]
        unsafe {
            let pid = signals::getpid();
            if pid > 0 {
                signals::kill(pid, 9);
            }
        }
    }

    Ok(())
}

#[cfg(unix)]
mod signals {
    use std::os::raw::c_int;
    extern "C" {
        pub fn kill(pid: c_int, sig: c_int) -> c_int;
        pub fn getpid() -> c_int;
    }
}
