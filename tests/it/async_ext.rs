use xshell::cmd;

use super::setup;

#[tokio::test]
async fn test_run_async() {
    let sh = setup();
    sh.change_dir("nonexistent");
    let err = cmd!(sh, "ls").run_async().await.unwrap_err();
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

#[tokio::test]
async fn test_read_async() {
    let sh = setup();

    let hello = "hello";
    let output = cmd!(sh, "xecho {hello}").read_async().await.unwrap();
    assert_eq!(output, "hello");
}

#[tokio::test]
async fn test_read_stderr_async() {
    let sh = setup();

    let output = cmd!(sh, "xecho -f -e snafu").ignore_status().read_stderr_async().await.unwrap();
    assert!(output.contains("snafu"));
}

#[tokio::test]
async fn output_with_ignore() {
    let sh = setup();

    let output = cmd!(sh, "xecho -e 'hello world!'").ignore_stdout().output().unwrap();
    assert_eq!(output.stderr, b"hello world!\n");
    assert_eq!(output.stdout, b"");

    let output = cmd!(sh, "xecho -e 'hello world!'").ignore_stderr().output().unwrap();
    assert_eq!(output.stdout, b"hello world!\n");
    assert_eq!(output.stderr, b"");

    let output =
        cmd!(sh, "xecho -e 'hello world!'").ignore_stdout().ignore_stderr().output().unwrap();
    assert_eq!(output.stdout, b"");
    assert_eq!(output.stderr, b"");
}

#[tokio::test]
async fn test_read_with_ignore() {
    let sh = setup();

    let stdout = cmd!(sh, "xecho -e 'hello world'").ignore_stdout().read().unwrap();
    assert!(stdout.is_empty());

    let stderr = cmd!(sh, "xecho -e 'hello world'").ignore_stderr().read_stderr().unwrap();
    assert!(stderr.is_empty());

    let stdout = cmd!(sh, "xecho -e 'hello world!'").ignore_stderr().read().unwrap();
    assert_eq!(stdout, "hello world!");

    let stderr = cmd!(sh, "xecho -e 'hello world!'").ignore_stdout().read_stderr().unwrap();
    assert_eq!(stderr, "hello world!");
}
