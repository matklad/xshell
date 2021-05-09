fn main() {
    if let Err(err) = try_main() {
        eprintln!("{}", err);
        std::process::exit(1);
    }
}

fn try_main() -> Result<(), std::io::Error> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    for key in &args {
        if let Some(v) = std::env::var_os(&key) {
            println!("{}={}", key, v.to_string_lossy());
        }
    }
    Ok(())
}
