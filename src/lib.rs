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
//!     }).expect("Error setting Ctrl-C handler");
//!     println!("Waiting for Ctrl-C...");
//!     while running.load(Ordering::SeqCst) {}
//!     println!("Got it! Exiting...");
//! }
//! ```

use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering, ATOMIC_BOOL_INIT};
use std::thread;

static INIT: AtomicBool = ATOMIC_BOOL_INIT;

#[derive(Debug)]
pub enum Error {
    Init(String),
    MultipleHandlers(String),
    SetHandler,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use std::error::Error;
        
        write!(f, "CtrlC Error: {}", self.description())
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Init(ref msg) => &msg,
            Error::MultipleHandlers(ref msg) => &msg,
            Error::SetHandler => "Error setting handler"
        }
}
}

#[cfg(unix)]
mod platform {
    extern crate libc;

    use ::Error;
    use self::libc::c_int;
    use self::libc::{signal, sighandler_t, SIGINT, SIG_ERR, EINTR};

    type PipeReadEnd = i32;
    type PipeWriteEnd = i32;
    pub static mut PIPE_FDS: (PipeReadEnd, PipeWriteEnd) = (-1, -1);

    pub use self::libc::{c_void, fcntl, FD_CLOEXEC, F_SETFD, pipe, read, write};

    #[cfg(feature = "termination")]
    use self::libc::SIGTERM;

    extern "C" {
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
        // Assuming this always succeeds. Can't really handle errors in any meaningful way.
        write(PIPE_FDS.1, &mut 0u8 as *mut _ as *mut c_void, 1);
    }

    #[cfg(feature = "termination")]
    #[inline]
    unsafe fn set_os_handler(handler: unsafe fn(c_int)) -> Result<(), Error> {
        if signal(SIGINT, ::std::mem::transmute::<_, sighandler_t>(handler)) == SIG_ERR {
            return Err(Error::SetHandler);
        }
        if signal(SIGTERM, ::std::mem::transmute::<_, sighandler_t>(handler)) == SIG_ERR {
            return Err(Error::SetHandler);
        }
        Ok(())
    }

    #[cfg(not(feature = "termination"))]
    #[inline]
    unsafe fn set_os_handler(handler: unsafe fn(c_int)) -> Result<(), Error> {
        if signal(SIGINT, ::std::mem::transmute::<_, sighandler_t>(handler)) == SIG_ERR {
            return Err(Error::SetHandler);
        }
        Ok(())
    }

    /// Register os signal handler, must be called before calling `block_ctrl_c()`.
    /// Should only be called once.
    #[inline]
    pub unsafe fn init_os_handler() -> Result<(), Error> {
        let mut fds = [0i32, 0];
        let pipe_fds = fds.as_mut_ptr();
        if pipe(pipe_fds) == -1 {
            return Err(Error::Init(format!("pipe failed with error {}", *errno_location())));
        }
        PIPE_FDS = (*pipe_fds.offset(0), *pipe_fds.offset(1));
        if fcntl(PIPE_FDS.0, F_SETFD, FD_CLOEXEC) == -1 {
            return Err(Error::Init(format!("fcntl failed with error {}", *errno_location())));
        }
        if fcntl(PIPE_FDS.1, F_SETFD, FD_CLOEXEC) == -1 {
            return Err(Error::Init(format!("fcntl failed with error {}", *errno_location())));
        }
        set_os_handler(os_handler)
    }

    /// Blocks until a Ctrl-C signal is received.
    #[inline]
    pub unsafe fn block_ctrl_c() {
        let mut buf = 0u8;
        loop {
            if read(PIPE_FDS.0, &mut buf as *mut _ as *mut c_void, 1) == -1 {
                assert_eq!(*errno_location(), EINTR);
            } else {
                break;
            }
        }
    }
}

#[cfg(windows)]
mod platform {
    extern crate winapi;
    extern crate kernel32;

    use ::Error;
    use self::kernel32::{SetConsoleCtrlHandler, CreateSemaphoreA, ReleaseSemaphore,
                         WaitForSingleObject};
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
    pub unsafe fn init_os_handler() -> Result<(), Error> {
        SEMAPHORE = CreateSemaphoreA(ptr::null_mut(), 0, MAX_SEM_COUNT, ptr::null());
        if SEMAPHORE.is_null() {
            return Err(Error::Init("CreateSemaphoreA failed".into()));
        }
        if SetConsoleCtrlHandler(Some(os_handler), TRUE) == FALSE {
            return Err(Error::SetHandler);
        }
        Ok(())
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
/// ctrlc::set_handler(|| println!("Hello world!")).expect("Error setting Ctrl-C handler");
/// ```
pub fn set_handler<F>(user_handler: F) -> Result<(), Error>
    where F: Fn() -> () + 'static + Send
{
    if INIT.swap(true, Ordering::SeqCst) != false {
        return Err(Error::MultipleHandlers("Ctrl-C signal handler already registered".into()));
    }

    unsafe {
        try!(platform::init_os_handler());
    }

    thread::spawn(move || {
        loop {
            unsafe {
                platform::block_ctrl_c();
            }
            user_handler();
        }
    });

    Ok(())
}

#[test]
fn test_multiple_handlers() {
    assert!(set_handler(|| {}).is_ok());
    assert!(set_handler(|| {}).is_err());
}
