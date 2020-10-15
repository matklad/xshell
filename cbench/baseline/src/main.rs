use std::process::Command;

fn main() {
    let mut cmd = Command::new("date");
    cmd.arg("--iso");
    let output = cmd.output().unwrap();
    if !output.status.success() {
        panic!("command `{:?}` failed: {}", cmd, output.status);
    }
    let stdout = String::from_utf8(output.stdout).unwrap();
    print!("today is: {}", stdout)
}
