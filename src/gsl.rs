//! Global shell lock
use std::{
    cell::Cell,
    mem::MaybeUninit,
    ptr,
    sync::Once,
    sync::{RwLock, RwLockReadGuard, RwLockWriteGuard},
};

#[derive(Debug)]
pub(crate) struct Guard(Option<Repr>);

#[derive(Debug)]
enum Repr {
    Read(RwLockReadGuard<'static, ()>),
    Write(RwLockWriteGuard<'static, ()>),
}

pub(crate) fn write() -> Guard {
    if matches!(CACHE.with(Cell::get), Cache::Write) {
        // this thread (and only this thread) can already write. don't try to
        // acquire another write guard again.
        return Guard(None);
    }
    // this thread has no writers. if it has readers, this will deadlock.
    let w_guard = static_rw_lock().write().unwrap_or_else(|err| err.into_inner());
    // if we got to here, we must not have any readers.
    assert!(matches!(CACHE.with(Cell::get), Cache::Read(0)));
    // note that we have a writer.
    CACHE.with(|it| it.set(Cache::Write));
    Guard(Some(Repr::Write(w_guard)))
}

pub(crate) fn read() -> Guard {
    match CACHE.with(Cell::get) {
        Cache::Write => {
            // this thread (and only this thread) can already write. it's safe
            // to allow this thread to read as well, because we won't have
            // concurrent reads and writes, because we're only working on this
            // thread.
            Guard(None)
        }
        Cache::Read(n) => {
            if n == 0 {
                // this thread has no readers or writers. try to acquire the
                // lock for reading.
                let r_guard = static_rw_lock().read().unwrap_or_else(|err| err.into_inner());
                // note that we now have 1 reader.
                CACHE.with(|it| it.set(Cache::Read(1)));
                Guard(Some(Repr::Read(r_guard)))
            } else {
                // this thread already has a read guard. don't try to acquire
                // one again. also, record that we have another reader.
                CACHE.with(|it| it.set(Cache::Read(n + 1)));
                Guard(None)
            }
        }
    }
}

fn static_rw_lock() -> &'static RwLock<()> {
    static mut LOCK: MaybeUninit<RwLock<()>> = MaybeUninit::uninit();
    static LOCK_INIT: Once = Once::new();
    unsafe {
        LOCK_INIT.call_once(|| ptr::write(LOCK.as_mut_ptr(), RwLock::new(())));
        &*LOCK.as_ptr()
    }
}

#[derive(Debug, Clone, Copy)]
enum Cache {
    Read(usize),
    Write,
}

thread_local! {
    static CACHE: Cell<Cache> = Cell::new(Cache::Read(0));
}

impl Drop for Guard {
    fn drop(&mut self) {
        match self.0 {
            Some(Repr::Read(_)) => CACHE.with(|it| {
                let n = match it.get() {
                    Cache::Read(n) => n,
                    Cache::Write => unreachable!(),
                };
                it.set(Cache::Read(n - 1));
            }),
            Some(Repr::Write(_)) => CACHE.with(|it| it.set(Cache::Read(0))),
            None => {}
        }
    }
}

#[test]
fn read_write_read() {
    eprintln!("get r1");
    let r1 = read();
    eprintln!("got r1");
    let h = std::thread::spawn(|| {
        eprintln!("get w1");
        let w1 = write();
        eprintln!("got w1");
        drop(w1);
        eprintln!("gave w1");
    });
    std::thread::sleep(std::time::Duration::from_millis(300));
    eprintln!("get r2");
    let r2 = read();
    eprintln!("got r2");
    drop(r1);
    eprintln!("gave r1");
    drop(r2);
    eprintln!("gave r2");
    h.join().unwrap();
}
