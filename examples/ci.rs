//! This CI script for `xshell`.
//!
//! It also serves as a real-world example, yay bootstrap!
use std::{env, process, thread, time::Duration, time::Instant};

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
    // Can't delete oneself on Windows.
    if !cfg!(windows) {
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
    let manifest = sh.read_file("./Cargo.toml")?;

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
    let tags = cmd!(sh, "git tag --list").read()?;
    let tag_exists = tags.contains(&tag);

    let current_branch = cmd!(sh, "git branch --show-current").read()?;

    if current_branch == "master" && !tag_exists {
        cmd!(sh, "git tag v{version}").run()?;

        let token = sh.var("CRATES_IO_TOKEN").unwrap_or("DUMMY_TOKEN".to_string());
        {
            let _p = sh.push_dir("xshell-macros");
            cmd!(sh, "cargo publish --token {token}").run()?;
            for _ in 0..100 {
                thread::sleep(Duration::from_secs(3));
                let err_msg =
                    cmd!(sh, "cargo install xshell-macros --version {version} --bin non-existing")
                        .ignore_status()
                        .read_stderr()?;

                let not_found = err_msg.contains("could not find ");
                let tried_installing = err_msg.contains("Installing");
                assert!(not_found ^ tried_installing);
                if tried_installing {
                    break;
                }
            }
        }
        cmd!(sh, "cargo publish --token {token}").run()?;
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
