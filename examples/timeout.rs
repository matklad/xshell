use std::time::Duration;

use anyhow::Result;
use xshell::{cmd, Shell};

fn main() -> Result<()> {
    let sh = Shell::new()?;
    let command = cmd!(sh, "sleep 5").timeout(Duration::from_secs(3));

    // Run the command with a timeout
    match command.run() {
        Ok(_) => println!("Command completed successfully."),
        Err(e) => eprintln!("Command failed: {}", e),
    }

    // Run the command with a timeout and get stdout
    match command.read() {
        Ok(output) => println!("Command output: {}", output),
        Err(e) => eprintln!("Command failed: {}", e),
    }

    // Run the command with a timeout and get stderr
    match command.read_stderr() {
        Ok(output) => println!("Command stderr: {}", output),
        Err(e) => eprintln!("Command failed: {}", e),
    }

    // Run the command with a timeout and get the full output
    match command.output() {
        Ok(output) => println!("Command completed successfully.{output:?}"),
        Err(e) => eprintln!("Command failed: {}", e),
    }

    Ok(())
}
