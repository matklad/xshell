# xshell: Making Rust a Better Bash

`xshell` provides a set of cross-platform utilities for writing cross-platform
and ergonomic "bash" scripts.

## Example

```rust
//! Clones a git repository and publishes it to crates.io.
use xshell::{cmd, Shell};

fn main() -> anyhow::Result<()> {
    let sh = Shell::new()?;

    let user = "matklad";
    let repo = "xshell";
    cmd!(sh, "git clone https://github.com/{user}/{repo}.git").run_echo()?;
    sh.set_current_dir(repo);

    let test_args = ["-Zunstable-options", "--report-time"];
    cmd!(sh, "cargo test -- {test_args...}").run_echo()?;

    let manifest = sh.read_file("Cargo.toml")?;
    let version = manifest
        .split_once("version = \"")
        .and_then(|it| it.1.split_once('\"'))
        .map(|it| it.0)
        .ok_or_else(|| anyhow::format_err!("can't find version field in the manifest"))?;

    cmd!(sh, "git tag {version}").run_echo()?;

    let dry_run = if sh.var("CI").is_ok() { None } else { Some("--dry-run") };
    cmd!(sh, "cargo publish {dry_run...}").run_echo()?;

    Ok(())
}
```

See [the docs](https://docs.rs/xshell) for more.

If you like the ideas behind xshell, you will enjoy [dax](https://github.com/dsherret/dax).
