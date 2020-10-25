use std::io;

fn main() {
    if let Err(err) = try_main() {
        eprintln!("error: {}", err);
        std::process::exit(1)
    }
}

fn try_main() -> io::Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    if args != ["date", "--iso"] {
        return Err(io::Error::new(io::ErrorKind::Other, "invalid args"));
    }
    println!("1982-06-25");
    Ok(())
}
