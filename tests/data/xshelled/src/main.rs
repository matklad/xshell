use xshell::{Shell, cmd};

fn main() {
    let sh = Shell::new().unwrap();
    let stdout = cmd!(sh, "echo hello world").read().unwrap();
    print!("{}\n", stdout)
}
