#![cfg(windows)]

use xshell::{cmd, Shell};

#[test]
fn echo() {
    let sh = Shell::new().unwrap();

    let res = cmd!(sh, "echo test").read().unwrap();
    assert_eq!(res, "test");
}

#[test]
fn npm() {
    let sh = Shell::new().unwrap();

    if cmd!(sh, "where npm.cmd").read().is_ok() {
        let script_shell = cmd!(sh, "npm get shell").read().unwrap();
        assert!(script_shell.ends_with(".exe"))
    }
}
