use std::collections::BTreeMap;

use xshell::{cmd, pushenv};

use crate::setup;

#[test]
fn test_env() {
    setup();

    let v1 = "xshell_test_123";
    let v2 = "xshell_test_456";

    assert_env(cmd!("echo_env {v1}").env(v1, "123"), &[(v1, Some("123"))]);

    assert_env(
        cmd!("echo_env {v1} {v2}").envs([(v1, "123"), (v2, "456")].iter().copied()),
        &[(v1, Some("123")), (v2, Some("456"))],
    );
    assert_env(
        cmd!("echo_env {v1} {v2}").envs([(v1, "123"), (v2, "456")].iter().copied()).env_remove(v2),
        &[(v1, Some("123")), (v2, None)],
    );
    assert_env(
        cmd!("echo_env {v1} {v2}")
            .envs([(v1, "123"), (v2, "456")].iter().copied())
            .env_remove("nothing"),
        &[(v1, Some("123")), (v2, Some("456"))],
    );

    let _g1 = pushenv(v1, "foobar");
    let _g2 = pushenv(v2, "quark");

    assert_env(cmd!("echo_env {v1} {v2}"), &[(v1, Some("foobar")), (v2, Some("quark"))]);

    assert_env(
        cmd!("echo_env {v1} {v2}").env(v1, "wombo"),
        &[(v1, Some("wombo")), (v2, Some("quark"))],
    );

    assert_env(cmd!("echo_env {v1} {v2}").env_remove(v1), &[(v1, None), (v2, Some("quark"))]);
    assert_env(
        cmd!("echo_env {v1} {v2}").env_remove(v1).env(v1, "baz"),
        &[(v1, Some("baz")), (v2, Some("quark"))],
    );
    assert_env(
        cmd!("echo_env {v1} {v2}").env(v1, "baz").env_remove(v1),
        &[(v1, None), (v2, Some("quark"))],
    );
}

#[test]
#[cfg(not(windows))]
fn test_env_clear() {
    setup();

    let v1 = "xshell_test_123";
    let v2 = "xshell_test_456";

    let echo_env = format!("./mock_bin/echo_env{}", std::env::consts::EXE_SUFFIX);

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

    assert_env(cmd!("{echo_env} {v1} {v2}").env_clear(), &[(v1, None), (v2, None)]);
    assert_env(
        cmd!("{echo_env} {v1} {v2}").env_clear().env(v1, "baz"),
        &[(v1, Some("baz")), (v2, None)],
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
            let (key, val) = split_once(line, '=').unwrap_or_else(|| {
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

// Remove when bumping MSRV to 1.52.0
fn split_once(line: &str, arg: char) -> Option<(&str, &str)> {
    let idx = line.find(arg)?;
    Some((&line[..idx], &line[idx + arg.len_utf8()..]))
}
