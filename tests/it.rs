use std::{ffi::OsStr, thread, time::Duration, time::Instant};

use xshell::{cmd, cwd, pushd, pushenv, read_file, rm_rf};

#[test]
fn smoke() {
    let output = cmd!("echo hello").read().unwrap();
    assert_eq!(output, "hello");
}

#[test]
fn multiline() {
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
    let hello = "hello";
    let output = cmd!("echo {hello}").read().unwrap();
    assert_eq!(output, "hello");
}

#[test]
fn program_interpolation() {
    let echo = "echo";
    let output = cmd!("{echo} hello").read().unwrap();
    assert_eq!(output, "hello");
}

#[test]
fn interpolation_concatenation() {
    let hello = "hello";
    let world = "world";
    let output = cmd!("echo {hello}-{world}").read().unwrap();
    assert_eq!(output, "hello-world")
}

#[test]
fn interpolation_move() {
    let hello = "hello".to_string();
    let output1 = cmd!("echo {hello}").read().unwrap();
    let output2 = cmd!("echo {hello}").read().unwrap();
    assert_eq!(output1, output2)
}

#[test]
fn interpolation_spat() {
    let a = &["hello", "world"];
    let b: &[&OsStr] = &[];
    let c = &["!".to_string()];
    let output = cmd!("echo {a...} {b...} {c...}").read().unwrap();
    assert_eq!(output, "hello world !")
}

#[test]
fn exit_status() {
    let err = cmd!("false").read().unwrap_err();
    assert_eq!(err.to_string(), "command `false` failed, exit code: 1");
}

#[test]
fn ignore_status() {
    let output = cmd!("false").ignore_status().read().unwrap();
    assert_eq!(output, "");
}

#[test]
fn read_stderr() {
    let output = cmd!("git fail").ignore_status().read_stderr().unwrap();
    assert!(output.contains("fail"));
}

#[test]
fn unknown_command() {
    let err = cmd!("nope no way").read().unwrap_err();
    assert_eq!(err.to_string(), "command not found: `nope`");
}

#[test]
fn args_with_spaces() {
    let hello_world = "hello world";
    let cmd = cmd!("echo {hello_world} 'hello world' hello world");
    assert_eq!(cmd.to_string(), r#"echo "hello world" "hello world" hello world"#)
}

#[test]
fn escape() {
    let output = cmd!("echo \\hello\\ '\\world\\'").read().unwrap();
    assert_eq!(output, r#"\hello\ \world\"#)
}

#[test]
fn stdin_redirection() {
    let lines = "\
foo
baz
bar
";
    let output = cmd!("sort").stdin(lines).read().unwrap();
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
    let current = cwd().unwrap();
    let dirname = current.file_name().unwrap();
    let _d = pushd("..").unwrap();
    let _d = pushd(dirname).unwrap();
    assert_eq!(cwd().unwrap(), current);
}

#[test]
fn test_pushd_lock() {
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
fn fixed_cost_compile_times() {
    let _p = pushd("cbench");
    let baseline = {
        let _p = pushd("baseline");
        compile_bench()
    };
    let xshelled = {
        let _p = pushd("xshelled");
        compile_bench()
    };
    let ratio = (xshelled.as_millis() as f64) / (baseline.as_millis() as f64);
    assert!(1.0 < ratio && ratio < 10.0)
}

fn compile_bench() -> Duration {
    let n = 5;
    let mut times = Vec::new();
    for _ in 0..n {
        rm_rf("./target").unwrap();
        let start = Instant::now();
        cmd!("cargo build").read().unwrap();
        times.push(start.elapsed());
    }
    times.sort();
    times.remove(0);
    times.pop();
    times.into_iter().sum::<Duration>()
}

#[test]
fn versions_match() {
    let read_version = |path: &str| {
        let text = read_file(path).unwrap();
        text.lines().find(|it| it.starts_with("version =")).unwrap().trim().to_string()
    };

    let v1 = read_version("./Cargo.toml");
    let v2 = read_version("./xshell-macros/Cargo.toml");
    assert_eq!(v1, v2);

    let cargo_toml = read_file("./Cargo.toml").unwrap();
    let dep = format!("xshell-macros = {{ {}", v1);
    assert!(cargo_toml.contains(&dep));
}

#[test]
fn formatting() {
    cmd!("cargo fmt --all -- --check").run().unwrap()
}

fn sleep_ms(ms: u64) {
    thread::sleep(std::time::Duration::from_millis(ms))
}
