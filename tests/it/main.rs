mod tidy;
mod env;
mod compile_failures;
mod timeout;

use std::{ffi::OsStr, path::Path};

use xshell::{cmd, Shell};

fn setup() -> Shell {
    static ONCE: std::sync::Once = std::sync::Once::new();

    let mut sh = Shell::new().unwrap();
    let xecho_src = sh.current_dir().join("./tests/data/xecho.rs");
    let xsleep_src = sh.current_dir().join("./tests/data/xsleep.rs");
    let target_dir = sh.current_dir().join("./target/");

    ONCE.call_once(|| {
        cmd!(sh, "rustc {xecho_src} --out-dir {target_dir}")
            .run()
            .unwrap_or_else(|err| panic!("failed to install binaries from mock_bin: {}", err));
        cmd!(sh, "rustc {xsleep_src} --out-dir {target_dir}")
            .run()
            .unwrap_or_else(|err| panic!("failed to install binaries from mock_bin: {}", err));
    });

    sh.set_var("PATH", target_dir);
    sh
}

#[test]
fn smoke() {
    let sh = setup();

    let pwd = "lol";
    let cmd = cmd!(sh, "xecho 'hello '{pwd}");
    println!("{}", cmd);
}

#[test]
fn into_command() {
    let sh = setup();
    let cmd = cmd!(sh, "git branch");
    let _ = cmd.to_command();
    let _: std::process::Command = cmd.into();
}

#[test]
fn multiline() {
    let sh = setup();

    let output = cmd!(
        sh,
        "
        xecho hello
        "
    )
    .read()
    .unwrap();
    assert_eq!(output, "hello");
}

#[test]
fn interpolation() {
    let sh = setup();

    let hello = "hello";
    let output = cmd!(sh, "xecho {hello}").read().unwrap();
    assert_eq!(output, "hello");
}

#[test]
fn program_interpolation() {
    let sh = setup();

    let echo = "xecho";
    let output = cmd!(sh, "{echo} hello").read().unwrap();
    assert_eq!(output, "hello");
}

#[test]
fn interpolation_concatenation() {
    let sh = setup();

    let hello = "hello";
    let world = "world";
    let output = cmd!(sh, "xecho {hello}-{world}").read().unwrap();
    assert_eq!(output, "hello-world");
}

#[test]
fn program_concatenation() {
    let sh = setup();

    let ho = "ho";
    let output = dbg!(cmd!(sh, "xec{ho} hello")).read().unwrap();
    assert_eq!(output, "hello");
}

#[test]
fn interpolation_move() {
    let sh = setup();

    let hello = "hello".to_string();
    let output1 = cmd!(sh, "xecho {hello}").read().unwrap();
    let output2 = cmd!(sh, "xecho {hello}").read().unwrap();
    assert_eq!(output1, output2)
}

#[test]
fn interpolation_spat() {
    let sh = setup();

    let a = &["hello", "world"];
    let b: &[&OsStr] = &[];
    let c = &["!".to_string()];
    let output = cmd!(sh, "xecho {a...} {b...} {c...}").read().unwrap();
    assert_eq!(output, "hello world !")
}

#[test]
fn splat_option() {
    let sh = setup();

    let a: Option<&OsStr> = None;
    let b = Some("hello");
    let output = cmd!(sh, "xecho {a...} {b...}").read().unwrap();
    assert_eq!(output, "hello")
}

#[test]
fn splat_idiom() {
    let sh = setup();

    let check = if true { &["--", "--check"][..] } else { &[] };
    let cmd = cmd!(sh, "cargo fmt {check...}");
    assert_eq!(cmd.to_string(), "cargo fmt -- --check");

    let dry_run = if true { Some("--dry-run") } else { None };
    let cmd = cmd!(sh, "cargo publish {dry_run...}");
    assert_eq!(cmd.to_string(), "cargo publish --dry-run");
}

#[test]
fn exit_status() {
    let sh = setup();

    let err = cmd!(sh, "xecho -f").read().unwrap_err();
    assert_eq!(
        err.to_string(),
        r#"command exited with non-zero code `xecho -f`: 1
stdout suffix:


stderr suffix:
other error

"#
    );
}

#[test]
#[cfg_attr(not(unix), ignore)]
fn exit_status_signal() {
    let sh = setup();

    let err = cmd!(sh, "xecho -s").read().unwrap_err();
    assert_eq!(
        err.to_string(),
        r#"command was terminated by a signal `xecho -s`: 9
stdout suffix:


"#
    );
}

#[test]
fn ignore_status() {
    let sh = setup();

    let output = cmd!(sh, "xecho -f").ignore_status().read().unwrap();
    assert_eq!(output, "");
}

#[test]
fn ignore_status_no_such_command() {
    let sh = setup();

    let err = cmd!(sh, "xecho-f").ignore_status().read().unwrap_err();
    assert_eq!(err.to_string(), "command not found: `xecho-f`");
}

#[test]
#[cfg_attr(not(unix), ignore)]
fn ignore_status_signal() {
    let sh = setup();

    let output = cmd!(sh, "xecho -s dead").ignore_status().read().unwrap();
    assert_eq!(output, "dead");
}

#[test]
fn read_stderr() {
    let sh = setup();

    let output = cmd!(sh, "xecho -f -e snafu").ignore_status().read_stderr().unwrap();
    assert!(output.contains("snafu"));
}

#[test]
fn unknown_command() {
    let sh = setup();

    let err = cmd!(sh, "nope no way").read().unwrap_err();
    assert_eq!(err.to_string(), "command not found: `nope`");
}

#[test]
fn args_with_spaces() {
    let sh = setup();

    let hello_world = "hello world";
    let cmd = cmd!(sh, "xecho {hello_world} 'hello world' hello world");
    assert_eq!(cmd.to_string(), r#"xecho "hello world" "hello world" hello world"#)
}

#[test]
fn escape() {
    let sh = setup();

    let output = cmd!(sh, "xecho \\hello\\ '\\world\\'").read().unwrap();
    assert_eq!(output, r#"\hello\ \world\"#)
}

#[test]
fn stdin_redirection() {
    let sh = setup();

    let lines = "\
foo
baz
bar
";
    let output = cmd!(sh, "xecho -i").stdin(lines).read().unwrap().replace("\r\n", "\n");
    assert_eq!(
        output,
        "\
foo
baz
bar
"
    )
}

#[test]
fn no_deadlock() {
    let sh = setup();

    let mut data = "All the work and now paly made Jack a dull boy.\n".repeat(1 << 20);
    data.pop();
    let res = cmd!(sh, "xecho -i").stdin(&data).read().unwrap();
    assert_eq!(data, res);
}

#[test]
fn test_push_dir() {
    let sh = setup();

    let d1 = sh.current_dir();
    {
        let sh = sh.with_current_dir("xshell-macros");
        let d2 = sh.current_dir();
        assert_eq!(d2, d1.join("xshell-macros"));
        {
            let sh = sh.with_current_dir("src");
            let d3 = sh.current_dir();
            assert_eq!(d3, d1.join("xshell-macros/src"));
        }
        let d4 = sh.current_dir();
        assert_eq!(d4, d1.join("xshell-macros"));
    }
    let d5 = sh.current_dir();
    assert_eq!(d5, d1);
}

#[test]
fn test_push_and_change_dir() {
    let sh = setup();

    let d1 = sh.current_dir();
    {
        let mut sh = sh.with_current_dir("xshell-macros");
        let d2 = sh.current_dir();
        assert_eq!(d2, d1.join("xshell-macros"));
        sh.set_current_dir("src");
        let d3 = sh.current_dir();
        assert_eq!(d3, d1.join("xshell-macros/src"));
    }
    let d5 = sh.current_dir();
    assert_eq!(d5, d1);
}

#[test]
fn push_dir_parent_dir() {
    let sh = setup();

    let current = sh.current_dir();
    let dirname = current.file_name().unwrap();
    let sh = sh.with_current_dir("..");
    let sh = sh.with_current_dir(dirname);
    assert_eq!(sh.current_dir().canonicalize().unwrap(), current.canonicalize().unwrap());
}

const VAR: &str = "SPICA";

#[test]
fn test_subshells_env() {
    let sh = setup();

    let e1 = sh.var_os(VAR);
    {
        let mut sh = sh.clone();
        sh.set_var(VAR, "1");
        let e2 = sh.var_os(VAR);
        assert_eq!(e2.as_deref(), Some("1".as_ref()));
        {
            let mut sh = sh.clone();
            let _e = sh.set_var(VAR, "2");
            let e3 = sh.var_os(VAR);
            assert_eq!(e3.as_deref(), Some("2".as_ref()));
        }
        let e4 = sh.var_os(VAR);
        assert_eq!(e4, e2);
    }
    let e5 = sh.var_os(VAR);
    assert_eq!(e5, e1);
}

#[test]
fn test_push_env_and_set_env_var() {
    let sh = setup();

    let e1 = sh.var_os(VAR);
    {
        let mut sh = sh.clone();
        sh.set_var(VAR, "1");
        let e2 = sh.var_os(VAR);
        assert_eq!(e2.as_deref(), Some("1".as_ref()));
        sh.set_var(VAR, "2");
        let e3 = sh.var_os(VAR);
        assert_eq!(e3.as_deref(), Some("2".as_ref()));
    }
    let e5 = sh.var_os(VAR);
    assert_eq!(e5, e1);
}

#[test]
fn test_copy_file() {
    let sh = setup();

    let path;
    {
        let tempdir = sh.create_temp_dir().unwrap();
        path = tempdir.path().to_path_buf();
        let foo = tempdir.path().join("foo.txt");
        let bar = tempdir.path().join("bar.txt");
        let dir = tempdir.path().join("dir");
        sh.write_file(&foo, "hello world").unwrap();
        sh.create_dir(&dir).unwrap();

        sh.copy_file(&foo, &bar).unwrap();
        assert_eq!(sh.read_file(&bar).unwrap(), "hello world");

        sh.copy_file_to_dir(&foo, &dir).unwrap();
        assert_eq!(sh.read_file(&dir.join("foo.txt")).unwrap(), "hello world");
        assert!(path.exists());
    }
    assert!(!path.exists());
}

#[test]
fn test_exists() {
    let mut sh = setup();
    let tmp = sh.create_temp_dir().unwrap();
    let _d = sh.set_current_dir(tmp.path());
    assert!(!sh.path_exists("foo.txt"));
    sh.write_file("foo.txt", "foo").unwrap();
    assert!(sh.path_exists("foo.txt"));
    assert!(!sh.path_exists("bar"));
    sh.create_dir("bar").unwrap();
    assert!(sh.path_exists("bar"));
    let _d = sh.set_current_dir("bar");
    assert!(!sh.path_exists("quz.rs"));
    sh.write_file("quz.rs", "fn main () {}").unwrap();
    assert!(sh.path_exists("quz.rs"));
    sh.remove_path("quz.rs").unwrap();
    assert!(!sh.path_exists("quz.rs"));
}

#[test]
fn write_makes_directory() {
    let sh = setup();

    let tempdir = sh.create_temp_dir().unwrap();
    let folder = tempdir.path().join("some/nested/folder/structure");
    sh.write_file(folder.join(".gitinclude"), "").unwrap();
    assert!(folder.exists());
}

#[test]
fn test_remove_path() {
    let mut sh = setup();

    let tempdir = sh.create_temp_dir().unwrap();
    sh.set_current_dir(tempdir.path());
    sh.write_file(Path::new("a/b/c.rs"), "fn main() {}").unwrap();
    assert!(tempdir.path().join("a/b/c.rs").exists());
    sh.remove_path("./a").unwrap();
    assert!(!tempdir.path().join("a/b/c.rs").exists());
    sh.remove_path("./a").unwrap();
}

#[test]
fn recovers_from_panics() {
    let sh = setup();

    let tempdir = sh.create_temp_dir().unwrap();
    let tempdir = tempdir.path().canonicalize().unwrap();

    let orig = sh.current_dir();

    std::panic::catch_unwind(|| {
        let sh = sh.with_current_dir(&tempdir);
        assert_eq!(sh.current_dir(), tempdir);
        std::panic::resume_unwind(Box::new(()));
    })
    .unwrap_err();

    assert_eq!(sh.current_dir(), orig);
    {
        let sh = sh.with_current_dir(&tempdir);
        assert_eq!(sh.current_dir(), tempdir);
    }
}

#[test]
fn string_escapes() {
    let sh = setup();

    assert_eq!(cmd!(sh, "\"hello\"").to_string(), "\"hello\"");
    assert_eq!(cmd!(sh, "\"\"\"asdf\"\"\"").to_string(), r##""""asdf""""##);
    assert_eq!(cmd!(sh, "\\\\").to_string(), r#"\\"#);
}

#[test]
fn nonexistent_current_directory() {
    let mut sh = setup();
    sh.set_current_dir("nonexistent");
    let err = cmd!(sh, "ls").run().unwrap_err();
    let message = err.to_string();
    if cfg!(unix) {
        assert!(message.contains("nonexistent"), "{message}");
        assert!(message.starts_with("failed to get current directory"));
        assert!(message.ends_with("No such file or directory (os error 2)"));
    } else {
        assert_eq!(
            message,
            "io error when running command `ls`: The directory name is invalid. (os error 267)"
        );
    }
}
