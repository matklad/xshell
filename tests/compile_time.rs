use std::time::{Duration, Instant};

use xshell::{cmd, Shell};

#[test]
fn fixed_cost_compile_times() {
    let mut sh = Shell::new().unwrap();

    let _p = sh.change_dir("tests/data");
    let baseline = compile_bench(&mut sh, "baseline");
    let _ducted = compile_bench(&sh, "ducted");
    let xshelled = compile_bench(&mut sh, "xshelled");
    let ratio = (xshelled.as_millis() as f64) / (baseline.as_millis() as f64);
    assert!(1.0 < ratio && ratio < 10.0);

    fn compile_bench(sh: &Shell, name: &str) -> Duration {
        let sh = sh.push_dir(name);
        let cargo_build = cmd!(sh, "cargo build -q");
        cargo_build.read().unwrap();

        let n = 5;
        let mut times = Vec::new();
        for _ in 0..n {
            sh.remove_path("./target").unwrap();
            let start = Instant::now();
            cargo_build.read().unwrap();
            let elapsed = start.elapsed();
            times.push(elapsed);
        }

        times.sort();
        times.remove(0);
        times.pop();
        let total = times.iter().sum::<Duration>();
        let average = total / (times.len() as u32);

        eprintln!("compiling {name}: {average:?}");

        total
    }
}
