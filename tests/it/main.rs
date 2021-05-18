mod env;

use std::{ffi::OsStr, thread, time::Duration, time::Instant};

use xshell::{
    cmd, cp, cwd, mkdir_p, mktemp_d, pushd, pushenv, read_dir, read_file, rm_rf, write_file,
};

#[test]
fn smoke() {
    setup();

    let pwd = "lol";
    let cmd = cmd!("echo 'hello '{pwd}");
    println!("{}", cmd);
}

#[test]
fn multiline() {
    setup();

    let output = cmd!(
        "
        echo hello
        "
    )
    .read()
    .unwrap();
    assert_eq!(output, "hello");
}

#[test]
fn interpolation() {
    setup();

    let hello = "hello";
    let output = cmd!("echo {hello}").read().unwrap();
    assert_eq!(output, "hello");
}

#[test]
fn program_interpolation() {
    setup();

    let echo = "echo";
    let output = cmd!("{echo} hello").read().unwrap();
    assert_eq!(output, "hello");
}

#[test]
fn interpolation_concatenation() {
    setup();

    let hello = "hello";
    let world = "world";
    let output = cmd!("echo {hello}-{world}").read().unwrap();
    assert_eq!(output, "hello-world")
}

#[test]
fn interpolation_move() {
    setup();

    let hello = "hello".to_string();
    let output1 = cmd!("echo {hello}").read().unwrap();
    let output2 = cmd!("echo {hello}").read().unwrap();
    assert_eq!(output1, output2)
}

#[test]
fn interpolation_spat() {
    setup();

    let a = &["hello", "world"];
    let b: &[&OsStr] = &[];
    let c = &["!".to_string()];
    let output = cmd!("echo {a...} {b...} {c...}").read().unwrap();
    assert_eq!(output, "hello world !")
}

#[test]
fn splat_idiom() {
    setup();

    let check = if true { &["--", "--check"][..] } else { &[] };
    let cmd = cmd!("cargo fmt {check...}");
    assert_eq!(cmd.to_string(), "cargo fmt -- --check");

    let dry_run = if true { Some("--dry-run") } else { None };
    let cmd = cmd!("cargo publish {dry_run...}");
    assert_eq!(cmd.to_string(), "cargo publish --dry-run");
}

#[test]
fn exit_status() {
    setup();

    let err = cmd!("false").read().unwrap_err();
    assert_eq!(err.to_string(), "command `false` failed, exit code: 1");
}

#[test]
fn ignore_status() {
    setup();

    let output = cmd!("false").ignore_status().read().unwrap();
    assert_eq!(output, "");
}

#[test]
fn read_stderr() {
    setup();

    let output = cmd!("git fail").ignore_status().read_stderr().unwrap();
    assert!(output.contains("fail"));
}

#[test]
fn unknown_command() {
    setup();

    let err = cmd!("nope no way").read().unwrap_err();
    assert_eq!(err.to_string(), "command not found: `nope`");
}

#[test]
fn args_with_spaces() {
    setup();

    let hello_world = "hello world";
    let cmd = cmd!("echo {hello_world} 'hello world' hello world");
    assert_eq!(cmd.to_string(), r#"echo "hello world" "hello world" hello world"#)
}

#[test]
fn escape() {
    setup();

    let output = cmd!("echo \\hello\\ '\\world\\'").read().unwrap();
    assert_eq!(output, r#"\hello\ \world\"#)
}

#[test]
fn stdin_redirection() {
    setup();

    let lines = "\
foo
baz
bar
";
    let output = cmd!("sort").stdin(lines).read().unwrap().replace("\r\n", "\n");
    assert_eq!(
        output,
        "\
bar
baz
foo"
    )
}

#[test]
fn test_pushd() {
    setup();

    let d1 = cwd().unwrap();
    {
        let _p = pushd("xshell-macros").unwrap();
        let d2 = cwd().unwrap();
        assert_eq!(d2, d1.join("xshell-macros"));
        {
            let _p = pushd("src").unwrap();
            let d3 = cwd().unwrap();
            assert_eq!(d3, d1.join("xshell-macros/src"));
        }
        let d4 = cwd().unwrap();
        assert_eq!(d4, d1.join("xshell-macros"));
    }
    let d5 = cwd().unwrap();
    assert_eq!(d5, d1);
}

#[test]
fn pushd_parent_dir() {
    setup();

    let current = cwd().unwrap();
    let dirname = current.file_name().unwrap();
    let _d = pushd("..").unwrap();
    let _d = pushd(dirname).unwrap();
    assert_eq!(cwd().unwrap(), current);
}

#[test]
fn test_pushd_lock() {
    setup();

    let t1 = thread::spawn(|| {
        let _p = pushd("cbench").unwrap();
        sleep_ms(20);
    });
    sleep_ms(10);

    let t2 = thread::spawn(|| {
        let _p = pushd("cbench").unwrap();
        sleep_ms(30);
    });

    t1.join().unwrap();
    t2.join().unwrap();
}

const VAR: &str = "SPICA";

#[test]
fn test_pushenv() {
    setup();

    let e1 = std::env::var_os(VAR);
    {
        let _e = pushenv(VAR, "1");
        let e2 = std::env::var_os(VAR);
        assert_eq!(e2, Some("1".into()));
        {
            let _e = pushenv(VAR, "2");
            let e3 = std::env::var_os(VAR);
            assert_eq!(e3, Some("2".into()));
        }
        let e4 = std::env::var_os(VAR);
        assert_eq!(e4, e2);
    }
    let e5 = std::env::var_os(VAR);
    assert_eq!(e5, e1);
}

#[test]
fn test_pushenv_lock() {
    setup();

    let t1 = thread::spawn(|| {
        let _e = pushenv(VAR, "hello");
        sleep_ms(20);
    });
    sleep_ms(10);

    let t2 = thread::spawn(|| {
        let _e = pushenv(VAR, "world");
        sleep_ms(30);
    });

    t1.join().unwrap();
    t2.join().unwrap();
}

#[test]
#[should_panic]
fn test_cannot_use_output_with_null() {
    let _ = cmd!("asdf").null().output();
}

#[test]
fn test_cp() {
    setup();

    let path;
    {
        let tempdir = mktemp_d().unwrap();
        path = tempdir.path().to_path_buf();
        let foo = tempdir.path().join("foo.txt");
        let bar = tempdir.path().join("bar.txt");
        let dir = tempdir.path().join("dir");
        write_file(&foo, "hello world").unwrap();
        mkdir_p(&dir).unwrap();

        cp(&foo, &bar).unwrap();
        assert_eq!(read_file(&bar).unwrap(), "hello world");

        cp(&foo, &dir).unwrap();
        assert_eq!(read_file(&dir.join("foo.txt")).unwrap(), "hello world");
        assert!(path.exists());
    }
    assert!(!path.exists());
}

fn check_failure(code: &str, err_msg: &str) {
    mkdir_p("./target/cf").unwrap();
    let _p = pushd("./target/cf").unwrap();

    write_file(
        "Cargo.toml",
        r#"
[package]
name = "cftest"
version = "0.0.0"
edition = "2018"
[workspace]

[lib]
path = "main.rs"

[dependencies]
xshell = { path = "../../" }
"#,
    )
    .unwrap();

    let snip = format!(
        "
use xshell::*;
pub fn f() {{
    {};
}}
",
        code
    );
    write_file("main.rs", snip).unwrap();

    let stderr = cmd!("cargo build").ignore_status().read_stderr().unwrap();
    assert!(
        stderr.contains(err_msg),
        "\n\nCompile fail fail!\n\nExpected:\n{}\n\nActual:\n{}\n",
        err_msg,
        stderr
    );
}

#[test]
fn write_makes_directory() {
    setup();

    let tempdir = mktemp_d().unwrap();
    let folder = tempdir.path().join("some/nested/folder/structure");
    write_file(folder.join(".gitinclude"), "").unwrap();
    assert!(folder.exists());
}

#[test]
fn test_compile_failures() {
    setup();

    check_failure("cmd!(92)", "expected a plain string literal");
    check_failure(r#"cmd!(r"raw")"#, "expected a plain string literal");

    check_failure(
        r#"cmd!("{echo.as_str()}")"#,
        "error: can only interpolate simple variables, got this expression instead: `echo.as_str()`",
    );

    check_failure(
        r#"cmd!("echo a{args...}")"#,
        "error: can't combine splat with concatenation, add spaces around `{args...}`",
    );
    check_failure(
        r#"cmd!("echo {args...}b")"#,
        "error: can't combine splat with concatenation, add spaces around `{args...}`",
    );
    check_failure(
        r#"cmd!("echo a{args...}b")"#,
        "error: can't combine splat with concatenation, add spaces around `{args...}`",
    );
    check_failure(r#"cmd!("")"#, "error: command can't be empty");
    check_failure(r#"cmd!("{cmd...}")"#, "error: can't splat program name");
    check_failure(r#"cmd!("echo 'hello world")"#, "error: unclosed `'` in command");
    check_failure(r#"cmd!("echo {hello world")"#, "error: unclosed `{` in command");

    check_failure(
        r#"
    let x = 92;
    cmd!("make -j {x}")"#,
        r#"cmd!("make -j {x}")"#,
    );

    check_failure(
        r#"
    let dry_run: fn() -> Option<&'static str> = || None;
    cmd!("make -j {dry_run...}")"#,
        r#"cmd!("make -j {dry_run...}")"#,
    );
}

#[test]
fn fixed_cost_compile_times() {
    setup();

    let _p = pushd("cbench").unwrap();
    let baseline = compile_bench("baseline");
    let _ducted = compile_bench("ducted");
    let xshelled = compile_bench("xshelled");
    let ratio = (xshelled.as_millis() as f64) / (baseline.as_millis() as f64);
    assert!(1.0 < ratio && ratio < 10.0)
}

fn compile_bench(name: &str) -> Duration {
    let _p = pushd(name).unwrap();
    cmd!("cargo build -q").read().unwrap();

    let n = 5;
    let mut times = Vec::new();
    for _ in 0..n {
        rm_rf("./target").unwrap();
        let start = Instant::now();
        cmd!("cargo build -q").read().unwrap();
        let elapsed = start.elapsed();
        times.push(elapsed);
    }

    times.sort();
    times.remove(0);
    times.pop();
    let total = times.iter().sum::<Duration>();
    let average = total / (times.len() as u32);

    eprintln!("compiling {}: {:?}", name, average);

    total
}

#[test]
fn versions_match() {
    setup();

    let read_version = |path: &str| {
        let text = read_file(path).unwrap();
        let vers = text.lines().find(|it| it.starts_with("version =")).unwrap();
        let vers = vers.splitn(2, '#').next().unwrap();
        vers.trim_start_matches("version =").trim().trim_matches('"').to_string()
    };

    let v1 = read_version("./Cargo.toml");
    let v2 = read_version("./xshell-macros/Cargo.toml");
    assert_eq!(v1, v2);

    let cargo_toml = read_file("./Cargo.toml").unwrap();
    let dep = format!("xshell-macros = {{ version = \"={}\",", v1);
    assert!(cargo_toml.contains(&dep));
}

#[test]
fn formatting() {
    setup();

    cmd!("cargo fmt --all -- --check").run().unwrap()
}

fn sleep_ms(ms: u64) {
    thread::sleep(std::time::Duration::from_millis(ms))
}

fn setup() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        if let Err(err) = install_mock_binaries() {
            panic!("failed to install binaries from mock_bin: {}", err)
        }
    });

    fn install_mock_binaries() -> xshell::Result<()> {
        let mock_bin = cwd()?.join("./mock_bin");
        let _d = pushd(&mock_bin);
        for path in read_dir(".")? {
            if path.extension().unwrap_or_default() == "rs" {
                cmd!("rustc {path}").run()?
            }
        }
        let old_path = std::env::var("PATH").unwrap_or_default();
        let new_path = {
            let mut path = std::env::split_paths(&old_path).collect::<Vec<_>>();
            path.insert(0, mock_bin);
            std::env::join_paths(path).unwrap()
        };
        std::env::set_var("PATH", new_path);
        Ok(())
    }
}
