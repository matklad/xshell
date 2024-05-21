use xshell::{cmd, Shell};

#[track_caller]
fn check(code: &str, err_msg: &str) {
    let mut sh = Shell::new().unwrap();
    let xshell_dir = sh.current_dir().to_owned();
    let temp_dir = sh.create_temp_dir().unwrap();
    sh.set_current_dir(temp_dir.path());

    let manifest = format!(
        r#"
[package]
name = "cftest"
version = "0.0.0"
edition = "2018"
[workspace]

[lib]
path = "main.rs"

[dependencies]
xshell = {{ path = {xshell_dir:?} }}
"#,
    );

    let snip = format!(
        "
use xshell::*;
pub fn f() {{
    let sh = Shell::new().unwrap();
    {code};
}}
"
    );

    sh.write_file("Cargo.toml", manifest).unwrap();
    sh.write_file("main.rs", snip).unwrap();

    let stderr = cmd!(sh, "cargo build").ignore_status().read_stderr().unwrap();
    assert!(
        stderr.contains(err_msg),
        "\n\nCompile fail fail!\n\nExpected:\n{}\n\nActual:\n{}\n",
        err_msg,
        stderr
    );
}

#[test]
fn not_a_string_literal() {
    check("cmd!(sh, 92)", "expected a plain string literal");
}

#[test]
fn not_raw_string_literal() {
    check(r#"cmd!(sh, r"raw")"#, "expected a plain string literal");
}

#[test]
fn interpolate_complex_expression() {
    check(
        r#"cmd!(sh, "{echo.as_str()}")"#,
        "error: can only interpolate simple variables, got this expression instead: `echo.as_str()`",
    );
}

#[test]
fn interpolate_splat_concat_prefix() {
    check(
        r#"cmd!(sh, "echo a{args...}")"#,
        "error: can't combine splat with concatenation, add spaces around `{args...}`",
    );
}

#[test]
fn interpolate_splat_concat_suffix() {
    check(
        r#"cmd!(sh, "echo {args...}b")"#,
        "error: can't combine splat with concatenation, add spaces around `{args...}`",
    );
}

#[test]
fn interpolate_splat_concat_mixfix() {
    check(
        r#"cmd!(sh, "echo a{args...}b")"#,
        "error: can't combine splat with concatenation, add spaces around `{args...}`",
    );
}

#[test]
fn empty_command() {
    check(r#"cmd!(sh, "")"#, "error: command can't be empty");
}

#[test]
fn spalt_program() {
    check(r#"cmd!(sh, "{cmd...}")"#, "error: can't splat program name");
}

#[test]
fn unclosed_quote() {
    check(r#"cmd!(sh, "echo 'hello world")"#, "error: unclosed `'` in command");
}

#[test]
fn unclosed_curly() {
    check(r#"cmd!(sh, "echo {hello world")"#, "error: unclosed `{` in command");
}

#[test]
fn interpolate_integer() {
    check(
        r#"
    let x = 92;
    cmd!(sh, "make -j {x}")"#,
        r#"is not implemented"#,
    );
}

#[test]
fn splat_fn_pointer() {
    check(
        r#"
    let dry_run: fn() -> Option<&'static str> = || None;
    cmd!(sh, "make -j {dry_run...}")"#,
        r#"is not implemented"#,
    );
}
