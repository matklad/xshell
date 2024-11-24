#![cfg(windows)]

use xshell::{cmd, Shell};

#[test]
fn npm() {
    let sh = Shell::new().unwrap();

    if cmd!(sh, "where npm").read().is_ok() {
        let script_shell = cmd!(sh, "npm get shell").read().unwrap();
        assert!(script_shell.ends_with(".exe"));

        let script_shell_explicit = cmd!(sh, "npm.cmd get shell").read().unwrap();
        assert_eq!(script_shell, script_shell_explicit);
    }
}

#[test]
fn overridden_child_path() {
    let sh = Shell::new().unwrap();

    if cmd!(sh, "where npm").read().is_ok() {
        // should succeed as sh contains its own `PATH`
        assert!(cmd!(sh, "npm get shell").env("PATH", ".").run().is_ok());
    }
}

#[test]
fn overridden_path() {
    let sh = Shell::new().unwrap();

    let _enc = sh.push_env("PATH", ".");

    if cmd!(sh, "where npm").read().is_ok() {
        // should fail as `PATH` is completely overridden
        assert!(cmd!(sh, "npm get shell").run().is_err());
    }
}
