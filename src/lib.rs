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
//! use ctrlc::CtrlC;
//! use std::sync::atomic::{AtomicBool, Ordering};
//! use std::sync::Arc;
//!
//! fn main() {
//!     let running = Arc::new(AtomicBool::new(true));
//!     let r = running.clone();
//!     CtrlC::set_handler(move || {
//!         r.store(false, Ordering::SeqCst);
//!     });
//!     println!("Waiting for Ctrl-C...");
//!     while running.load(Ordering::SeqCst) {}
//!     println!("Got it! Exiting...");
//! }
//! ```

use std::sync::atomic::Ordering;
use std::thread;
use std::time;

mod features {
    use std::sync::atomic::{AtomicBool, ATOMIC_BOOL_INIT};
    pub static DONE: AtomicBool = ATOMIC_BOOL_INIT;
}
use self::features::*;

#[cfg(unix)]
mod platform {
    extern crate libc;
    use self::libc::c_int;
    use self::libc::sighandler_t;
    use self::libc::SIGINT;
    use self::libc::signal;
    use std::sync::atomic::Ordering;

    pub fn handler(_: c_int) {
        super::features::DONE.store(true, Ordering::Relaxed);
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
    /// Sets up the signal handler for Ctrl-C using default polling rate of 100ms.
    /// # Example
    /// ```
    /// # use ctrlc::CtrlC;
    /// CtrlC::set_handler(|| println!("Hello world!"));
    /// ```
    pub fn set_handler<F>(user_handler: F)
        where F: Fn() -> () + 'static + Send
    {
        CtrlC::set_handler_with_polling_rate(user_handler, time::Duration::from_millis(100));
    }

    /// Sets up the signal handler for Ctrl-C using a custom polling rate.
    /// The polling rate is the amount of time the internal spinloop of CtrlC sleeps between
    /// iterations. Because condition variables are not safe to use inside a signal handler,
    /// CtrlC (from version 1.1.0) uses a spinloop and an atomic boolean instead.
    ///
    /// Normally you should use `set_handler` instead, but if the default rate of  100 milliseconds
    /// is too fast or too slow for you, you can use this routine instead to set your own.
    /// # Example
    /// ```
    /// # use std::time::Duration;
    /// # use ctrlc::CtrlC;
    /// CtrlC::set_handler_with_polling_rate(
    ///     || println!("Hello world!"),
    ///     Duration::from_millis(10)
    /// );
    /// ```
    pub fn set_handler_with_polling_rate<F>(user_handler: F, polling_rate: time::Duration)
        where F: Fn() -> () + 'static + Send
    {
        unsafe {
            set_os_handler(handler);
        }
        thread::spawn(move || {
            loop {
                if DONE.compare_and_swap(true, false, Ordering::Relaxed) {
                    user_handler();
                }
                thread::sleep(polling_rate);
            }
        });
    }
}
