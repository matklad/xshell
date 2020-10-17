//! Global shell lock
use std::{
    cell::Cell,
    mem::MaybeUninit,
    ptr,
    sync::Once,
    sync::{Mutex, MutexGuard},
};

pub(crate) struct Guard {
    guard: Option<MutexGuard<'static, ()>>,
}

pub(crate) fn lock() -> Guard {
    if LOCKED.with(|it| it.get()) {
        return Guard { guard: None };
    }

    let guard = unsafe {
        MUTEX_INIT.call_once(|| ptr::write(MUTEX.as_mut_ptr(), Mutex::new(())));
        (*MUTEX.as_ptr()).lock().unwrap()
    };
    LOCKED.with(|it| it.set(true));
    Guard { guard: Some(guard) }
}

static mut MUTEX: MaybeUninit<Mutex<()>> = MaybeUninit::uninit();
static MUTEX_INIT: Once = Once::new();
thread_local! {
    pub static LOCKED: Cell<bool> = Cell::new(false);
}

impl Drop for Guard {
    fn drop(&mut self) {
        if self.guard.is_some() {
            LOCKED.with(|it| it.set(false))
        }
    }
}
