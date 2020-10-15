use xshell::cmd;

fn main() {
    let date = cmd!("date --iso").read().unwrap();
    print!("today is: {}\n", date)
}
