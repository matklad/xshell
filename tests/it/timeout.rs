use std::time::Duration;

use xshell::{cmd, Shell};

#[test]
fn test_run_timeout_success() {
    let sh = Shell::new().unwrap();
    let command = cmd!(sh, "sleep 1"); // Command that sleeps for 1 second

    // Run the command with a timeout of 3 seconds
    let result = command.run_timeout(Duration::from_secs(3));
    assert!(result.is_ok(), "Command should complete successfully within the timeout");
}

#[test]
fn test_run_timeout_failure() {
    let sh = Shell::new().unwrap();
    let command = cmd!(sh, "sleep 5"); // Command that sleeps for 5 seconds

    // Run the command with a timeout of 3 seconds
    let result = command.run_timeout(Duration::from_secs(3));
    assert!(result.is_err(), "Command should fail due to timeout");
}

#[test]
fn test_read_timeout_success() {
    let sh = Shell::new().unwrap();
    let command = cmd!(sh, "echo Hello, world!"); // Command that prints a message

    // Run the command with a timeout of 3 seconds and read stdout
    let result = command.read_timeout(Duration::from_secs(3));
    assert!(result.is_ok(), "Command should complete successfully within the timeout");
    assert_eq!(result.unwrap(), "Hello, world!");
}

#[test]
fn test_read_timeout_failure() {
    let sh = Shell::new().unwrap();
    let command = cmd!(sh, "sleep 5"); // Command that sleeps for 5 seconds

    // Run the command with a timeout of 3 seconds and read stdout
    let result = command.read_timeout(Duration::from_secs(3));
    assert!(result.is_err(), "Command should fail due to timeout");
}

#[test]
fn test_read_stderr_timeout_success() {
    let sh = Shell::new().unwrap();
    let command = cmd!(sh, "sh -c 'echo Error message 1>&2'"); // Command that prints an error message to stderr

    // Run the command with a timeout of 3 seconds and read stderr
    let result = command.read_stderr_timeout(Duration::from_secs(3));
    assert!(result.is_ok(), "Command should complete successfully within the timeout");
    assert_eq!(result.unwrap(), "Error message");
}

#[test]
fn test_read_stderr_timeout_failure() {
    let sh = Shell::new().unwrap();
    let command = cmd!(sh, "sleep 5"); // Command that sleeps for 5 seconds

    // Run the command with a timeout of 3 seconds and read stderr
    let result = command.read_stderr_timeout(Duration::from_secs(3));
    assert!(result.is_err(), "Command should fail due to timeout");
}

#[test]
fn test_output_timeout_success() {
    let sh = Shell::new().unwrap();
    let command = cmd!(sh, "echo Hello, world!"); // Command that prints a message

    // Run the command with a timeout of 3 seconds and get the full output
    let result = command.output_timeout(Duration::from_secs(3));
    assert!(result.is_ok(), "Command should complete successfully within the timeout");
    let output = result.unwrap();
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "Hello, world!");
}

#[test]
fn test_output_timeout_failure() {
    let sh = Shell::new().unwrap();
    let command = cmd!(sh, "sleep 5"); // Command that sleeps for 5 seconds

    // Run the command with a timeout of 3 seconds and get the full output
    let result = command.output_timeout(Duration::from_secs(3));
    assert!(result.is_err(), "Command should fail due to timeout");
}
