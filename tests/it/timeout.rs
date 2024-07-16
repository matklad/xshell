use crate::setup;
use std::time::Duration;

use xshell::cmd;

#[test]
fn test_run_timeout_success() {
    let sh = setup();
    let command = cmd!(sh, "xsleep 1"); // Command that xsleeps for 1 second

    // Run the command with a timeout of 3 seconds
    let result = command.timeout(Duration::from_secs(3)).run();
    assert!(result.is_ok(), "Command should complete successfully within the timeout");
}

#[test]
fn test_run_timeout_failure() {
    let sh = setup();
    let command = cmd!(sh, "xsleep 5"); // Command that xsleeps for 5 seconds

    // Run the command with a timeout of 3 seconds
    let result = command.timeout(Duration::from_secs(3)).run();
    assert!(result.is_err(), "Command should fail due to timeout");
}

#[test]
fn test_read_timeout_success() {
    let sh = setup();
    let command = cmd!(sh, "xecho Hello, world!"); // Command that prints a message

    // Run the command with a timeout of 3 seconds and read stdout
    let result = command.timeout(Duration::from_secs(3)).read();
    assert!(result.is_ok(), "Command should complete successfully within the timeout");
    assert_eq!(result.unwrap(), "Hello, world!");
}

#[test]
fn test_read_timeout_failure() {
    let sh = setup();
    let command = cmd!(sh, "xsleep 5"); // Command that xsleeps for 5 seconds

    // Run the command with a timeout of 3 seconds and read stdout
    let result = command.timeout(Duration::from_secs(3)).read();
    assert!(result.is_err(), "Command should fail due to timeout");
}

#[test]
fn test_read_stderr_timeout_success() {
    let sh = setup();
    let command = cmd!(sh, "xecho -e Error message"); // Command that prints an error message to stderr

    // Run the command with a timeout of 3 seconds and read stderr
    let result = command.timeout(Duration::from_secs(3)).read_stderr();
    assert!(result.is_ok(), "Command should complete successfully within the timeout");
    assert_eq!(result.unwrap(), "Error message");
}

#[test]
fn test_read_stderr_timeout_failure() {
    let sh = setup();
    let command = cmd!(sh, "xsleep 5"); // Command that xsleeps for 5 seconds

    // Run the command with a timeout of 3 seconds and read stderr
    let result = command.timeout(Duration::from_secs(3)).read_stderr();
    assert!(result.is_err(), "Command should fail due to timeout");
}

#[test]
fn test_output_timeout_success() {
    let sh = setup();
    let command = cmd!(sh, "xecho Hello, world!"); // Command that prints a message

    // Run the command with a timeout of 3 seconds and get the full output
    let result = command.timeout(Duration::from_secs(3)).output();
    assert!(result.is_ok(), "Command should complete successfully within the timeout");
    let output = result.unwrap();
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "Hello, world!");
}

#[test]
fn test_output_timeout_failure() {
    let sh = setup();
    let command = cmd!(sh, "xsleep 5"); // Command that xsleeps for 5 seconds

    // Run the command with a timeout of 3 seconds and get the full output
    let result = command.timeout(Duration::from_secs(3)).output();
    assert!(result.is_err(), "Command should fail due to timeout");
}
