use duct::cmd;

fn main() {
    let stdout = cmd!("echo", "hello", "world").read().unwrap();
    print!("{}\n", stdout)
}
