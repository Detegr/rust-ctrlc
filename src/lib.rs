// Copyright (c) 2015 CtrlC developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

//! A simple easy to use wrapper around Ctrl-C.
//! # Example
//! ```no_run
//! extern crate ctrlc;
//! use std::sync::atomic::{AtomicBool, Ordering};
//! use std::sync::Arc;
//!
//! fn main() {
//!     let running = Arc::new(AtomicBool::new(true));
//!     let r = running.clone();
//!     ctrlc::set_handler(move || {
//!         r.store(false, Ordering::SeqCst);
//!     });
//!     println!("Waiting for Ctrl-C...");
//!     while running.load(Ordering::SeqCst) {}
//!     println!("Got it! Exiting...");
//! }
//! ```

use std::sync::atomic::{AtomicBool, Ordering, ATOMIC_BOOL_INIT};
use std::thread;

static INIT: AtomicBool = ATOMIC_BOOL_INIT;

#[cfg(unix)]
mod platform {
    extern crate libc;

    use self::libc::c_int;
    use self::libc::{signal, sighandler_t, SIGINT, SIG_ERR, EINTR};
    use self::libc::{sem_t, sem_init, sem_wait, sem_post};

    #[cfg(feature = "termination")]
    use self::libc::SIGTERM;

    static mut SEMAPHORE: *mut sem_t = 0 as *mut sem_t;

    extern {
        #[cfg_attr(any(target_os = "macos",
                       target_os = "ios",
                       target_os = "freebsd"),
                   link_name = "__error")]
        #[cfg_attr(target_os = "dragonfly",
                   link_name = "__dfly_error")]
        #[cfg_attr(any(target_os = "openbsd",
                       target_os = "bitrig",
                       target_os = "android"),
                   link_name = "__errno")]
        #[cfg_attr(target_os = "linux",
                   link_name = "__errno_location")]
        fn errno_location() -> *mut c_int;
    }

    unsafe fn os_handler(_: c_int) {
        // sem_post() is async-signal-safe. It will only fail
        // when the semaphore counter has reached its maximum value or
        // if the semaphore is invalid, we can therefore safely
        // ignore return value.
        sem_post(SEMAPHORE);
    }

    #[cfg(feature = "termination")]
    #[inline]
    unsafe fn set_os_handler(handler: unsafe fn(c_int)) {
        assert_ne!(signal(SIGINT,  ::std::mem::transmute::<_, sighandler_t>(handler)), SIG_ERR);
        assert_ne!(signal(SIGTERM, ::std::mem::transmute::<_, sighandler_t>(handler)), SIG_ERR);
    }

    #[cfg(not(feature = "termination"))]
    #[inline]
    unsafe fn set_os_handler(handler: unsafe fn(c_int)) {
        assert_ne!(signal(SIGINT, ::std::mem::transmute::<_, sighandler_t>(handler)), SIG_ERR);
    }

    /// Register os signal handler, must be called before calling block_ctrl_c().
    /// Should only be called once.
    #[inline]
    pub unsafe fn init_os_handler() {
        SEMAPHORE = Box::into_raw(Box::new(::std::mem::uninitialized::<sem_t>()));
        assert_ne!(sem_init(SEMAPHORE, 0, 0), -1);
        set_os_handler(os_handler);
    }

    /// Blocks until a Ctrl-C signal is received.
    #[inline]
    pub unsafe fn block_ctrl_c() {
        loop {
            if sem_wait(SEMAPHORE) == 0 {
                break;
            } else {
                // Retry if errno is EINTR
                assert_eq!(*errno_location(), EINTR);
            }
        }
    }
}

#[cfg(windows)]
mod platform {
    extern crate winapi;
    extern crate kernel32;

    use self::kernel32::{SetConsoleCtrlHandler, CreateSemaphoreA, ReleaseSemaphore, WaitForSingleObject};
    use self::winapi::{HANDLE, BOOL, DWORD, TRUE, FALSE, INFINITE, WAIT_OBJECT_0, c_long};

    use std::ptr;

    const MAX_SEM_COUNT: c_long = 255;
    static mut SEMAPHORE: HANDLE = 0 as HANDLE;

    unsafe extern "system" fn os_handler(_: DWORD) -> BOOL {
        // ReleaseSemaphore() should only fail when the semaphore
        // counter has reached its maximum value or if the semaphore
        // is invalid, we can therefore safely ignore return value.
        ReleaseSemaphore(SEMAPHORE, 1, ptr::null_mut());
        TRUE
    }

    /// Register os signal handler, must be called before calling block_ctrl_c().
    /// Should only be called once.
    #[inline]
    pub unsafe fn init_os_handler() {
        SEMAPHORE = CreateSemaphoreA(ptr::null_mut(), 0, MAX_SEM_COUNT, ptr::null());
        assert!(!SEMAPHORE.is_null());
        assert_ne!(SetConsoleCtrlHandler(Some(os_handler), TRUE), FALSE);
    }

    /// Blocks until a Ctrl-C signal is received.
    #[inline]
    pub unsafe fn block_ctrl_c() {
        assert_eq!(WaitForSingleObject(SEMAPHORE, INFINITE), WAIT_OBJECT_0);
    }
}

/// Sets up the signal handler for Ctrl-C.
/// # Example
/// ```
/// ctrlc::set_handler(|| println!("Hello world!"));
/// ```
pub fn set_handler<F>(user_handler: F)
    where F: Fn() -> () + 'static + Send
{
    assert!(INIT.swap(true, Ordering::SeqCst) == false, "Ctrl-C signal handler already registered");

    unsafe {
        platform::init_os_handler();
    }

    thread::spawn(move || {
        loop {
            unsafe {
                platform::block_ctrl_c();
            }
            user_handler();
        }
    });
}
