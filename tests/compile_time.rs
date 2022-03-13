use std::time::{Duration, Instant};

use xshell::{cmd, Shell};

#[test]
fn fixed_cost_compile_times() {
    let sh = Shell::new().unwrap();

    let _p = sh.push_dir("cbench");
    let baseline = compile_bench(&sh, "baseline");
    // FIXME: Don't have internet rn, can't compile duct :-(
    // let _ducted = compile_bench(&sh, "ducted");
    let xshelled = compile_bench(&sh, "xshelled");
    let ratio = (xshelled.as_millis() as f64) / (baseline.as_millis() as f64);
    assert!(1.0 < ratio && ratio < 10.0);

    fn compile_bench(sh: &Shell, name: &str) -> Duration {
        let _p = sh.push_dir(name);
        cmd!(sh, "cargo build -q").read().unwrap();

        let n = 5;
        let mut times = Vec::new();
        for _ in 0..n {
            sh.remove_path("./target").unwrap();
            let start = Instant::now();
            cmd!(sh, "cargo build -q").read().unwrap();
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
