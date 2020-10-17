//! This CI script for `xshell`.
//!
//! It also serves as a real-world example, yay bootstrap!
use std::{env, process, time::Instant};

use xshell::{cmd, pushd, read_file, rm_rf, Result};

fn main() {
    if let Err(err) = try_main() {
        eprintln!("{}", err);
        process::exit(1);
    }
}

fn try_main() -> Result<()> {
    let manifest = read_file("./Cargo.toml")?;
    rm_rf("./target")?;

    {
        let _s = Section::new("BUILD");
        cmd!("cargo test --workspace --no-run").run()?;
    }

    {
        let _s = Section::new("TEST");
        cmd!("cargo test --workspace").run()?;
    }

    {
        let _s = Section::new("PUBLISH");

        let version = manifest
            .lines()
            .find_map(|line| {
                let words = line.split_ascii_whitespace().collect::<Vec<_>>();
                match words.as_slice() {
                    [n, "=", v, ..] if n.trim() == "version" => {
                        assert!(v.starts_with('"') && v.ends_with('"'));
                        return Some(&v[1..v.len() - 1]);
                    }
                    _ => None,
                }
            })
            .unwrap();

        let tag = format!("v{}", version);
        let tags = cmd!("git tag --list").read()?;
        let tag_exists = tags.contains(&tag);

        let current_branch = cmd!("git branch --show-current").read()?;

        let dry_run =
            if tag_exists || &current_branch != "master" { &["--dry-run"][..] } else { &[] };

        if dry_run.is_empty() {
            cmd!("git tag v{version}").run()?;
        }

        let token = env::var("CRATES_IO_TOKEN").unwrap_or("DUMMY_TOKEN".to_string());
        {
            let _p = pushd("xshell-macros")?;
            cmd!("cargo publish --token {token} {dry_run...}").run()?;
            std::thread::sleep(std::time::Duration::from_secs(60));
        }
        cmd!("cargo publish --token {token} {dry_run...}").run()?;
        cmd!("git push --tags {dry_run...}").run()?;
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
