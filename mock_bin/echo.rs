fn main() {
    let mut space = "";
    for arg in std::env::args().skip(1) {
        print!("{}{}", space, arg);
        space = " ";
    }
    println!();
}
