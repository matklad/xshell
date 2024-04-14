//! Clones a git repository and publishes it to crates.io.
use xshell::{cmd, Shell};

fn main() -> anyhow::Result<()> {
    let mut sh = Shell::new()?;

    let user = "matklad";
    let repo = "xshell";
    cmd!(sh, "git clone https://github.com/{user}/{repo}.git").run()?;
    sh.change_dir(repo);

    let test_args = ["-Zunstable-options", "--report-time"];
    cmd!(sh, "cargo test -- {test_args...}").run()?;

    let manifest = sh.read_file("Cargo.toml")?;
    let version = manifest
        .split_once("version = \"")
        .and_then(|it| it.1.split_once('\"'))
        .map(|it| it.0)
        .ok_or_else(|| anyhow::format_err!("can't find version field in the manifest"))?;

    cmd!(sh, "git tag {version}").run()?;

    let dry_run = if sh.var("CI").is_ok() { None } else { Some("--dry-run") };
    cmd!(sh, "cargo publish {dry_run...}").run()?;

    Ok(())
}
