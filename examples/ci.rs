//! This CI script for `xshell`.
//!
//! It also serves as a real-world example, yay bootstrap!
use std::{env, process, time::Instant};

use xshell::{cmd, Result, Shell};

fn main() {
    if let Err(err) = try_main() {
        eprintln!("{}", err);
        process::exit(1);
    }
}

fn try_main() -> Result<()> {
    let sh = Shell::new()?;
    if env::args().nth(1).as_deref() == Some("publish") {
        publish(&sh)
    } else {
        test(&sh)
    }
}

fn test(sh: &Shell) -> Result<()> {
    // A good setup for CI is to compile & run in two steps, to get separate feedback for compile
    // time and run time. However, because we are using the crate itself to run CI, if we are
    // running this, we've already compiled a  bunch of stuff. Originally we tried to `rm -rf
    // .target`, but we also observed weird SIGKILL: 9 errors on mac. Perhaps its our self-removal?
    // Let's scope it only to linux (windows won't work, bc one can not remove oneself there).
    if cfg!(unix) {
        sh.remove_path("./target")?;
    }

    {
        let _s = Section::new("BUILD");
        cmd!(sh, "cargo test --workspace --no-run").run()?;
    }

    {
        let _s = Section::new("TEST");
        cmd!(sh, "cargo test --workspace").run()?;
    }
    Ok(())
}

fn publish(sh: &Shell) -> Result<()> {
    let _s = Section::new("PUBLISH");

    let pkgid = cmd!(sh, "cargo pkgid").read()?;
    let (_path, version) = pkgid.rsplit_once('#').unwrap();

    let tag = format!("v{}", version);
    let tags = cmd!(sh, "git tag --list").read()?;
    let tag_exists = tags.split_ascii_whitespace().any(|it| it == &tag);

    let current_branch = cmd!(sh, "git branch --show-current").read()?;

    if current_branch == "master" && !tag_exists {
        // Could also just use `CARGO_REGISTRY_TOKEN` environmental variable.
        let token = sh.var("CRATES_IO_TOKEN").unwrap_or("DUMMY_TOKEN".to_string());
        cmd!(sh, "git tag v{version}").run()?;
        cmd!(sh, "cargo publish --token {token} --package xshell-macros").run()?;
        cmd!(sh, "cargo publish --token {token} --package xshell").run()?;
        cmd!(sh, "git push --tags").run()?;
    }
    Ok(())
}

struct Section {
    name: &'static str,
    start: Instant,
}

impl Section {
    fn new(name: &'static str) -> Section {
        println!("::group::{}", name);
        let start = Instant::now();
        Section { name, start }
    }
}

impl Drop for Section {
    fn drop(&mut self) {
        println!("{}: {:.2?}", self.name, self.start.elapsed());
        println!("::endgroup::");
    }
}
