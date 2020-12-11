//! This CI script for `xshell`.
//!
//! It also serves as a real-world example, yay bootstrap!
use std::{env, process, thread, time::Duration, time::Instant};

use xshell::{cmd, cwd, pushd, pushenv, read_dir, read_file, rm_rf, Result};

fn main() {
    if let Err(err) = try_main() {
        eprintln!("{}", err);
        process::exit(1);
    }
}

fn try_main() -> Result<()> {
    if env::args().nth(1).as_deref() == Some("publish") {
        publish()
    } else {
        test()
    }
}

fn test() -> Result<()> {
    // Can't delete oneself on Windows.
    if !cfg!(windows) {
        rm_rf("./target")?;
    }

    let path_with_mock_bin = {
        let _s = Section::new("INSTALL_MOCK_BIN");

        let mock_bin = cwd()?.join("./mock_bin");
        let _d = pushd(&mock_bin);
        for path in read_dir(".")? {
            if path.extension().unwrap_or_default() == "rs" {
                cmd!("rustc {path}").run()?
            }
        }
        let path = env::var("PATH").unwrap_or_default();
        let mut path = env::split_paths(&path).collect::<Vec<_>>();
        path.insert(0, mock_bin);
        env::join_paths(path).unwrap()
    };
    let _e = pushenv("PATH", path_with_mock_bin);

    {
        let _s = Section::new("BUILD");
        cmd!("cargo test --workspace --no-run").run()?;
    }

    {
        let _s = Section::new("TEST");
        cmd!("cargo test --workspace").run()?;
    }
    Ok(())
}

fn publish() -> Result<()> {
    let _s = Section::new("PUBLISH");
    let manifest = read_file("./Cargo.toml")?;

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

    if current_branch == "master" && !tag_exists {
        cmd!("git tag v{version}").run()?;

        let token = env::var("CRATES_IO_TOKEN").unwrap_or("DUMMY_TOKEN".to_string());
        {
            let _p = pushd("xshell-macros")?;
            cmd!("cargo publish --token {token}").run()?;
            for _ in 0..100 {
                thread::sleep(Duration::from_secs(3));
                let err_msg =
                    cmd!("cargo install xshell-macros --version {version} --bin non-existing")
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
        cmd!("cargo publish --token {token}").run()?;
        cmd!("git push --tags").run()?;
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
