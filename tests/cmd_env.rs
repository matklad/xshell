use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use xshell::{cmd, pushd, pushenv};

#[test]
fn test_env() {
    let echo_env = dbg!(echo_env_path());

    let v1 = "xshell_test_123";
    let v2 = "xshell_test_456";

    assert_env(cmd!("{echo_env} {v1}").env(v1, "123"), &[(v1, Some("123"))]);

    assert_env(
        cmd!("{echo_env} {v1} {v2}").envs([(v1, "123"), (v2, "456")].iter().copied()),
        &[(v1, Some("123")), (v2, Some("456"))],
    );
    assert_env(
        cmd!("{echo_env} {v1} {v2}")
            .envs([(v1, "123"), (v2, "456")].iter().copied())
            .env_remove(v2),
        &[(v1, Some("123")), (v2, None)],
    );
    assert_env(
        cmd!("{echo_env} {v1} {v2}")
            .envs([(v1, "123"), (v2, "456")].iter().copied())
            .env_remove("nothing"),
        &[(v1, Some("123")), (v2, Some("456"))],
    );
    assert_env(
        cmd!("{echo_env} {v1} {v2}").envs([(v1, "123"), (v2, "456")].iter().copied()).env_clear(),
        &[(v1, None), (v2, None)],
    );
    assert_env(
        cmd!("{echo_env} {v1} {v2}")
            .envs([(v1, "123"), (v2, "456")].iter().copied())
            .env_clear()
            .env(v1, "789"),
        &[(v1, Some("789")), (v2, None)],
    );

    let _g1 = pushenv(v1, "foobar");
    let _g2 = pushenv(v2, "quark");

    assert_env(cmd!("{echo_env} {v1} {v2}"), &[(v1, Some("foobar")), (v2, Some("quark"))]);

    assert_env(
        cmd!("{echo_env} {v1} {v2}").env(v1, "wombo"),
        &[(v1, Some("wombo")), (v2, Some("quark"))],
    );

    assert_env(cmd!("{echo_env} {v1} {v2}").env_clear(), &[(v1, None), (v2, None)]);
    assert_env(cmd!("{echo_env} {v1} {v2}").env_remove(v1), &[(v1, None), (v2, Some("quark"))]);
    assert_env(
        cmd!("{echo_env} {v1} {v2}").env_clear().env(v1, "baz"),
        &[(v1, Some("baz")), (v2, None)],
    );
    assert_env(
        cmd!("{echo_env} {v1} {v2}").env_remove(v1).env(v1, "baz"),
        &[(v1, Some("baz")), (v2, Some("quark"))],
    );
    assert_env(
        cmd!("{echo_env} {v1} {v2}").env(v1, "baz").env_remove(v1),
        &[(v1, None), (v2, Some("quark"))],
    );
    assert_env(cmd!("{echo_env} {v1} {v2}").env(v1, "baz").env_clear(), &[(v1, None), (v2, None)]);
}

#[track_caller]
fn assert_env(echo_env_cmd: xshell::Cmd, want_env: &[(&str, Option<&str>)]) {
    let output = echo_env_cmd.output().unwrap();
    let env = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            let (key, val) = line.split_once('=').unwrap_or_else(|| {
                panic!("failed to parse line from `echo_env` output: {:?}", line)
            });
            (key.to_owned(), val.to_owned())
        })
        .collect::<BTreeMap<_, _>>();
    check_env(&env, want_env);
}

#[track_caller]
fn check_env(env: &BTreeMap<String, String>, wanted_env: &[(&str, Option<&str>)]) {
    let mut failed = false;
    let mut seen = env.clone();
    for &(k, val) in wanted_env {
        match (seen.remove(k), val) {
            (Some(env_v), Some(want_v)) if env_v == want_v => {}
            (None, None) => {}
            (have, want) => {
                eprintln!("mismatch on env var {:?}: have `{:?}`, want `{:?}` ", k, have, want);
                failed = true;
            }
        }
    }
    for (k, v) in seen {
        eprintln!("Unexpected env key {:?} (value: {:?})", k, v);
        failed = true;
    }
    assert!(
        !failed,
        "env didn't match (see stderr for cleaner output):\nsaw: {:?}\n\nwanted: {:?}",
        env, wanted_env,
    );
}

fn find_on_path(cmd: &str) -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("PATH") {
        std::env::split_paths(&path).map(|p| p.join(cmd)).find(|p| p.exists())
    } else {
        None
    }
}

fn echo_env_path() -> PathBuf {
    // If the command is on our PATH already, use it directly.
    if let Some(path) = find_on_path("echo_env".as_ref()) {
        return path.into();
    }
    // Otherwise, compile it (once, if needed) — this is a bit involved, but
    // keeps `cargo test` working without extra setup.
    static COMPILE_ONCE: std::sync::Once = std::sync::Once::new();
    COMPILE_ONCE.call_once(|| maybe_compile_mock_bin("echo_env"));
    Path::new("./mock_bin").join("echo_env")
}

fn maybe_compile_mock_bin(cmd: &str) {
    let _g = pushd("mock_bin");
    let bin_path = Path::new(cmd);
    // If there's no executable, or if the source's modification time is more
    // recent than that of the executable, rebuild.
    let need_rebuild = !bin_path.exists() || {
        let src_path = Path::new(&format!("{}.rs", cmd)).to_owned();
        assert!(src_path.exists(), "No such file: mock_bin/{}.rs", cmd);
        let bin_mtime = bin_path.metadata().and_then(|meta| meta.modified());
        let src_mtime = src_path.metadata().and_then(|meta| meta.modified());
        match (bin_mtime, src_mtime) {
            (Ok(bin), Ok(src)) => src >= bin,
            _ => true,
        }
    };
    if !need_rebuild {
        return;
    }
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".into());
    cmd!("{rustc} {cmd}.rs").run().unwrap();
    let bin_path = bin_path.canonicalize().unwrap_or_else(|_| bin_path.to_owned());
    assert!(bin_path.exists(), "After compiling, {} still doesn't exist", bin_path.display(),);
}
