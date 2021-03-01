//! Global shell lock.

use std::{
    cell::Cell,
    mem::MaybeUninit,
    ptr,
    sync::Once,
    sync::{RwLock, RwLockReadGuard, RwLockWriteGuard},
};

/// If, on the same thread, there are multiple calls to [`read`] or [`write`],
/// then the `Guard`s returned should be dropped in the reverse order that they
/// were acquired.
///
/// If this is violated, e.g. in
///
/// ```ignore
/// let w1 = write();
/// let _w2 = write();
/// drop(w1);
/// // try to use a global resource with write privileges
/// ```
///
/// then things won't turn out well, because `_w2` doesn't actually contain a
/// lock guard.
#[derive(Debug)]
pub(crate) struct Guard(Option<Repr>);

#[derive(Debug)]
enum Repr {
    Read(RwLockReadGuard<'static, ()>),
    Write(RwLockWriteGuard<'static, ()>),
}

/// Returns a [`Guard`] for write access to global resources.
pub(crate) fn write() -> Guard {
    match CACHE.with(Cell::get) {
        Cache::Write => {
            // this thread (and only this thread) can already write. don't try to
            // acquire another write guard.
            Guard(None)
        }
        Cache::Read(readers) => {
            assert_eq!(
                readers, 0,
                "calling write() with an active read guard on the same thread would deadlock"
            );
            let w_guard = static_rw_lock().write().unwrap_or_else(|err| err.into_inner());
            // note that we have a writer.
            CACHE.with(|it| it.set(Cache::Write));
            Guard(Some(Repr::Write(w_guard)))
        }
    }
}

/// Returns a [`Guard`] for read access to global resources.
pub(crate) fn read() -> Guard {
    match CACHE.with(Cell::get) {
        Cache::Write => {
            // this thread (and only this thread) can already write. it's safe
            // to allow this thread to read as well, because we won't have
            // concurrent reads and writes, because we're only working on this
            // thread.
            Guard(None)
        }
        Cache::Read(readers) => {
            if readers == 0 {
                // this thread has no readers or writers. try to acquire the
                // lock for reading.
                let r_guard = static_rw_lock().read().unwrap_or_else(|err| err.into_inner());
                // note that we now have 1 reader.
                CACHE.with(|it| it.set(Cache::Read(1)));
                Guard(Some(Repr::Read(r_guard)))
            } else {
                // this thread can already read. don't try to acquire another
                // read guard. also, note that we have another reader.
                CACHE.with(|it| it.set(Cache::Read(readers + 1)));
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
                    Cache::Write => unreachable!("had both a reader and a writer"),
                };
                it.set(Cache::Read(n - 1));
            }),
            Some(Repr::Write(_)) => CACHE.with(|it| {
                assert_eq!(it.get(), Cache::Write);
                it.set(Cache::Read(0));
            }),
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

#[test]
fn write_read() {
    let _w = write();
    let _r = read();
}

#[test]
#[should_panic(
    expected = "calling write() with an active read guard on the same thread would deadlock"
)]
fn read_write() {
    let _r = read();
    let _w = write();
}
