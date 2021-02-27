use std::{
    sync::{Arc, RwLock},
    thread,
    time::Duration,
};

#[test]
fn u_dead() {
    let lock = Arc::new(RwLock::new(()));

    let r1 = thread::spawn({
        let lock = Arc::clone(&lock);
        move || {
            let _rg = lock.read();
            eprintln!("r1/1");
            sleep(1000);

            let _rg = lock.read();
            eprintln!("r1/2");

            sleep(5000);
        }
    });
    sleep(100);
    let w = thread::spawn({
        let lock = Arc::clone(&lock);
        move || {
            let _wg = lock.write();
            eprintln!("w");
        }
    });
    sleep(100);
    let r2 = thread::spawn({
        let lock = Arc::clone(&lock);
        move || {
            let _rg = lock.read();
            eprintln!("r2");
            sleep(2000);
        }
    });

    r1.join().unwrap();
    r2.join().unwrap();
    w.join().unwrap();
}

fn sleep(ms: u64) {
    std::thread::sleep(Duration::from_millis(ms))
}
