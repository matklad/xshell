use std::process::Command;

fn main() {
    let mut cmd = Command::new("echo");
    cmd.arg("hello");
    cmd.arg("world");
    let output = cmd.output().unwrap();
    if !output.status.success() {
        panic!("command `{:?}` failed: {}", cmd, output.status);
    }
    let stdout = String::from_utf8(output.stdout).unwrap();
    print!("{}", stdout)
}
