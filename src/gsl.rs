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
    if LOCKED.with(|it| it.get()) {
        return Guard(None);
    }

    let w_guard = static_rw_lock().write().unwrap_or_else(|err| err.into_inner());
    LOCKED.with(|it| it.set(true));
    Guard(Some(Repr::Write(w_guard)))
}

pub(crate) fn read() -> Guard {
    if LOCKED.with(|it| it.get()) {
        return Guard(None);
    }

    let r_guard = static_rw_lock().read().unwrap_or_else(|err| err.into_inner());
    Guard(Some(Repr::Read(r_guard)))
}

fn static_rw_lock() -> &'static RwLock<()> {
    static mut LOCK: MaybeUninit<RwLock<()>> = MaybeUninit::uninit();
    static LOCK_INIT: Once = Once::new();
    unsafe {
        LOCK_INIT.call_once(|| ptr::write(LOCK.as_mut_ptr(), RwLock::new(())));
        &*LOCK.as_ptr()
    }
}

thread_local! {
    static LOCKED: Cell<bool> = Cell::new(false);
}

impl Drop for Guard {
    fn drop(&mut self) {
        if matches!(self.0, Some(Repr::Write(_))) {
            LOCKED.with(|it| it.set(false))
        }
    }
}
