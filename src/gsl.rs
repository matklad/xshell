//! Global shell lock
use std::{
    cell::Cell,
    mem::MaybeUninit,
    ptr,
    sync::Once,
    sync::{RwLock, RwLockReadGuard, RwLockWriteGuard},
};

pub(crate) struct Guard {
    r_guard: Option<RwLockReadGuard<'static, ()>>,
    w_guard: Option<RwLockWriteGuard<'static, ()>>,
}

pub(crate) fn write() -> Guard {
    if LOCKED.with(|it| it.get()) {
        return Guard { r_guard: None, w_guard: None };
    }

    let w_guard = static_rw_lock().write().unwrap_or_else(|err| err.into_inner());
    LOCKED.with(|it| it.set(true));
    Guard { w_guard: Some(w_guard), r_guard: None }
}

pub(crate) fn read() -> Guard {
    if LOCKED.with(|it| it.get()) {
        return Guard { r_guard: None, w_guard: None };
    }

    let r_guard = static_rw_lock().read().unwrap_or_else(|err| err.into_inner());
    Guard { w_guard: None, r_guard: Some(r_guard) }
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
        if self.w_guard.is_some() {
            LOCKED.with(|it| it.set(false))
        }
        let _ = self.r_guard;
    }
}
