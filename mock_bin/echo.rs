fn main() {
    if let Err(err) = try_main() {
        eprintln!("error: {}", err);
        std::process::exit(1)
    }
}

fn try_main() -> std::io::Result<()> {
    let mut space = "";
    for arg in std::env::args().skip(1) {
        print!("{}{}", space, arg);
        space = " ";
    }
    println!();
    Ok(())
}
