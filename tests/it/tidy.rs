use xshell::{cmd, Shell};

#[test]
fn versions_match() {
    let sh = Shell::new().unwrap();

    let read_version = |path: &str| {
        let text = sh.read_file(path).unwrap();
        let vers = text.lines().find(|it| it.starts_with("version =")).unwrap();
        let vers = vers.splitn(2, '#').next().unwrap();
        vers.trim_start_matches("version =").trim().trim_matches('"').to_string()
    };

    let v1 = read_version("./Cargo.toml");

    let cargo_toml = sh.read_file("./Cargo.toml").unwrap();
    let dep = format!("xshell-macros = {{ version = \"={}\",", v1);
    assert!(cargo_toml.contains(&dep));
}

#[test]
fn formatting() {
    let sh = Shell::new().unwrap();

    cmd!(sh, "cargo fmt --all -- --check").run().unwrap();
}

#[test]
fn current_version_in_changelog() {
    let sh = Shell::new().unwrap();
    let _p = sh.push_dir(env!("CARGO_MANIFEST_DIR"));
    let changelog = sh.read_file("CHANGELOG.md").unwrap();
    let current_version_header = format!("## {}", env!("CARGO_PKG_VERSION"));
    assert_eq!(changelog.lines().filter(|&line| line == current_version_header).count(), 1);
}
