use std::{io::stdin, process::exit};

// TODO: switch to `std::io::IsTerminal` when MSRV >= 1.70.0
use is_terminal::IsTerminal;

fn main() {
    if stdin().is_terminal() {
        println!("Stdin is terminal");
        exit(0);
    } else {
        println!("Stdin is not terminal");
        exit(1);
    }
}
