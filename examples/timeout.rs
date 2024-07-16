use std::time::Duration;

use anyhow::Result;
use xshell::{cmd, Shell};

fn main() -> Result<()> {
    let sh = Shell::new()?;
    let command = cmd!(sh, "sleep 5");

    // Run the command with a timeout
    match command.run_timeout(Duration::from_secs(3)) {
        Ok(_) => println!("Command completed successfully."),
        Err(e) => eprintln!("Command failed: {}", e),
    }

    // Run the command with a timeout and get stdout
    match command.read_timeout(Duration::from_secs(3)) {
        Ok(output) => println!("Command output: {}", output),
        Err(e) => eprintln!("Command failed: {}", e),
    }

    // Run the command with a timeout and get stderr
    match command.read_stderr_timeout(Duration::from_secs(3)) {
        Ok(output) => println!("Command stderr: {}", output),
        Err(e) => eprintln!("Command failed: {}", e),
    }

    // Run the command with a timeout and get the full output
    match command.output_timeout(Duration::from_secs(3)) {
        Ok(output) => println!("Command completed successfully.{output:?}"),
        Err(e) => eprintln!("Command failed: {}", e),
    }

    Ok(())
}
