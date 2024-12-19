use std::{
    io::{stdin, IsTerminal},
    process::exit,
};

fn main() {
    if stdin().is_terminal() {
        println!("Stdin is terminal");
        exit(0);
    } else {
        println!("Stdin is not terminal");
        exit(1);
    }
}
