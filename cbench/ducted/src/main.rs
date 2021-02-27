use duct::cmd;

fn main() {
    let date = cmd!("date", "+%Y-%m-%d").read().unwrap();
    print!("today is: {}\n", date)
}
