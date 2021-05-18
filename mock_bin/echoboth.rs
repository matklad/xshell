fn main() {
    let mut space = "";
    for arg in std::env::args().skip(1) {
        eprint!("{}{}", space, arg);
        print!("{}{}", space, arg);
        space = " ";
    }
    eprintln!();
    println!();
}
