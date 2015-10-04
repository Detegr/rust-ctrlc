//! A simple easy to use wrapper around Ctrl-C signal.

#![cfg_attr(feature="nightly", feature(static_condvar))]
#![cfg_attr(feature="nightly", feature(static_mutex))]

#[cfg(feature="stable")]
#[macro_use]
extern crate lazy_static;

use std::sync::atomic::Ordering;

#[cfg(feature="nightly")]
mod features {
    use std::sync::atomic::{AtomicBool, ATOMIC_BOOL_INIT};
    use std::sync::{StaticCondvar, CONDVAR_INIT, StaticMutex, MUTEX_INIT};
    pub static CVAR: StaticCondvar = CONDVAR_INIT;
    pub static MUTEX: StaticMutex = MUTEX_INIT;
    pub static DONE: AtomicBool = ATOMIC_BOOL_INIT;
}
#[cfg(not(feature="nightly"))]
mod features {
    use std::sync::atomic::{AtomicBool, ATOMIC_BOOL_INIT};
    use std::sync::{Condvar, Mutex};
    lazy_static! {
        pub static ref CVAR: Condvar = Condvar::new();
        pub static ref MUTEX: Mutex<bool> = Mutex::new(false);
    }
    pub static DONE: AtomicBool = ATOMIC_BOOL_INIT;
}
use self::features::*;

#[cfg(unix)]
mod platform {
    extern crate libc;
    use self::libc::c_int;
    use self::libc::types::os::common::posix01::sighandler_t;
    use self::libc::consts::os::posix88::SIGINT;
    use self::libc::funcs::posix01::signal::signal;
    use std::sync::atomic::Ordering;

    #[repr(C)]
    pub fn handler(_: c_int) {
        super::features::DONE.store(true, Ordering::Relaxed);
        super::features::CVAR.notify_all();
    }
    #[inline]
    pub unsafe fn set_os_handler(handler: fn(c_int)) {
        signal(SIGINT, ::std::mem::transmute::<_, sighandler_t>(handler));
    }
}
#[cfg(windows)]
mod platform {
    extern crate winapi;
    extern crate kernel32;
    use self::kernel32::SetConsoleCtrlHandler;
    use self::winapi::{BOOL, DWORD, TRUE};
    use std::sync::atomic::Ordering;

    pub unsafe extern "system" fn handler(_: DWORD) -> BOOL {
        super::features::DONE.store(true, Ordering::Relaxed);
        super::features::CVAR.notify_all();
        TRUE
    }
    #[inline]
    pub unsafe fn set_os_handler(handler: unsafe extern "system" fn(DWORD) -> BOOL) {
        SetConsoleCtrlHandler(Some(handler), TRUE);
    }
}
use self::platform::*;

pub struct CtrlC;
impl CtrlC {
    /// Sets up the signal handler for Ctrl-C
    /// # Example
    /// ```
    /// use ctrlc::CtrlC;
    /// CtrlC::set_handler(|| println!("Hello world!"));
    /// ```
    pub fn set_handler<F: Fn() -> () + 'static + Send>(user_handler: F) -> () {
        unsafe {
            set_os_handler(handler);
        }
        ::std::thread::spawn(move || {
            loop {
                if !DONE.load(Ordering::Relaxed) {
                    let _ = CVAR.wait(MUTEX.lock().unwrap());
                    DONE.store(false, Ordering::Relaxed);
                }
                user_handler();
            }
        });
    }
}
