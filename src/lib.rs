// Copyright (c) 2015 CtrlC developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

#![warn(missing_docs)]

//! Cross platform handling of Ctrl-C signals.
//!
//! [HandlerRoutine]:https://msdn.microsoft.com/en-us/library/windows/desktop/ms683242.aspx
//!
//! [set_handler()](fn.set_handler.html) allows setting a handler closure which is executed on
//! `Ctrl+C`. On Unix, this corresponds to a `SIGINT` signal. On windows, `Ctrl+C` corresponds to
//! [`CTRL_C_EVENT`][HandlerRoutine] or [`CTRL_BREAK_EVENT`][HandlerRoutine].
//!
//! Setting a handler will start a new dedicated signal handling thread where we
//! execute the handler each time we receive a `Ctrl+C` signal. There can only be
//! one handler, you would typically set one at the start of your program.
//!
//! # Example
//! ```no_run
//! extern crate ctrlc;
//! use std::sync::atomic::{AtomicBool, Ordering};
//! use std::sync::Arc;
//!
//! fn main() {
//!     let running = Arc::new(AtomicBool::new(true));
//!     let r = running.clone();
//!
//!     ctrlc::set_handler(move || {
//!         r.store(false, Ordering::SeqCst);
//!     }).expect("Error setting Ctrl-C handler");
//!
//!     println!("Waiting for Ctrl-C...");
//!     while running.load(Ordering::SeqCst) {}
//!     println!("Got it! Exiting...");
//! }
//! ```
//!
//! # Handling SIGTERM
//! Handling of `SIGTERM` can be enabled with `termination` feature. If this is enabled,
//! the handler specified by `set_handler()` will be executed for both `SIGINT` and `SIGTERM`.
//!

use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering, ATOMIC_BOOL_INIT};
use std::thread;

static INIT: AtomicBool = ATOMIC_BOOL_INIT;

/// Ctrl-C error.
#[derive(Debug)]
pub enum Error {
    /// Ctrl-C signal handler already registered.
    MultipleHandlers,
    /// Unexpected system error.
    System(std::io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use std::error::Error;
        write!(f, "Ctrl-C error: {}", self.description())
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::MultipleHandlers => "Ctrl-C signal handler already registered",
            Error::System(_) => "Unexpected system error",
        }
    }

    fn cause(&self) -> Option<&std::error::Error> {
        match *self {
            Error::System(ref e) => Some(e),
            _ => None,
        }
    }
}

#[cfg(unix)]
mod platform {
    extern crate nix;

    use super::Error;
    use self::nix::unistd;
    use std::os::unix::io::RawFd;
    use std::io;

    static mut PIPE: (RawFd, RawFd) = (-1, -1);

    extern "C" fn os_handler(_: nix::c_int) {
        // Assuming this always succeeds. Can't really handle errors in any meaningful way.
        unsafe {
            unistd::write(PIPE.1, &[0u8]).is_ok();
        }
    }

    /// Register os signal handler.
    ///
    /// Must be called before calling [`block_ctrl_c()`](fn.block_ctrl_c.html)
    /// and should only be called once.
    ///
    /// # Errors
    /// Will return an error if a system error occurred.
    ///
    #[inline]
    pub unsafe fn init_os_handler() -> Result<(), Error> {
        use self::nix::fcntl;
        use self::nix::sys::signal;

        PIPE = unistd::pipe2(fcntl::O_CLOEXEC).map_err(
            |e| Error::System(e.into()),
        )?;

        let close_pipe = |e: nix::Error| -> Error {
            unistd::close(PIPE.1).is_ok();
            unistd::close(PIPE.0).is_ok();
            Error::System(e.into())
        };

        // Make sure we never block on write in the os handler.
        if let Err(e) = fcntl::fcntl(PIPE.1, fcntl::FcntlArg::F_SETFL(fcntl::O_NONBLOCK)) {
            return Err(close_pipe(e));
        }

        let handler = signal::SigHandler::Handler(os_handler);
        let new_action =
            signal::SigAction::new(handler, signal::SA_RESTART, signal::SigSet::empty());

        let _old = match signal::sigaction(signal::Signal::SIGINT, &new_action) {
            Ok(old) => old,
            Err(e) => return Err(close_pipe(e)),
        };

        #[cfg(feature = "termination")]
        match signal::sigaction(signal::Signal::SIGTERM, &new_action) {
            Ok(_) => {}
            Err(e) => {
                signal::sigaction(signal::Signal::SIGINT, &_old).unwrap();
                return Err(close_pipe(e));
            }
        }

        // TODO: Maybe throw an error if old action is not SigDfl.

        Ok(())
    }

    /// Blocks until a Ctrl-C signal is received.
    ///
    /// Must be called after calling [`init_os_handler()`](fn.init_os_handler.html).
    ///
    /// # Errors
    /// Will return an error if a system error occurred.
    ///
    #[inline]
    pub unsafe fn block_ctrl_c() -> Result<(), Error> {
        let mut buf = [0u8];

        // TODO: Can we safely convert the pipe fd into a std::io::Read
        // with std::os::unix::io::FromRawFd, this would handle EINTR
        // and everything for us.
        loop {
            match unistd::read(PIPE.0, &mut buf[..]) {
                Ok(1) => break,
                Ok(_) => return Err(Error::System(io::ErrorKind::UnexpectedEof.into()).into()),
                Err(nix::Error::Sys(nix::Errno::EINTR)) => {}
                Err(e) => return Err(Error::System(e.into())),
            }
        }

        Ok(())
    }
}

#[cfg(windows)]
mod platform {
    extern crate winapi;
    extern crate kernel32;

    use super::Error;
    use self::winapi::{HANDLE, BOOL, DWORD, TRUE, FALSE, c_long};
    use std::ptr;
    use std::io;

    const MAX_SEM_COUNT: c_long = 255;
    static mut SEMAPHORE: HANDLE = 0 as HANDLE;

    unsafe extern "system" fn os_handler(_: DWORD) -> BOOL {
        // Assuming this always succeeds. Can't really handle errors in any meaningful way.
        kernel32::ReleaseSemaphore(SEMAPHORE, 1, ptr::null_mut());
        TRUE
    }

    /// Register os signal handler.
    ///
    /// Must be called before calling [`block_ctrl_c()`](fn.block_ctrl_c.html)
    /// and should only be called once.
    ///
    /// # Errors
    /// Will return an error if a system error occurred.
    ///
    #[inline]
    pub unsafe fn init_os_handler() -> Result<(), Error> {
        SEMAPHORE = kernel32::CreateSemaphoreA(ptr::null_mut(), 0, MAX_SEM_COUNT, ptr::null());
        if SEMAPHORE.is_null() {
            return Err(Error::System(io::Error::last_os_error()));
        }

        if kernel32::SetConsoleCtrlHandler(Some(os_handler), TRUE) == FALSE {
            let e = io::Error::last_os_error();
            kernel32::CloseHandle(SEMAPHORE);
            SEMAPHORE = 0 as HANDLE;
            return Err(Error::System(e));
        }

        Ok(())
    }

    /// Blocks until a Ctrl-C signal is received.
    ///
    /// Must be called after calling [`init_os_handler()`](fn.init_os_handler.html).
    ///
    /// # Errors
    /// Will return an error if a system error occurred.
    ///
    #[inline]
    pub unsafe fn block_ctrl_c() -> Result<(), Error> {
        use self::winapi::{INFINITE, WAIT_OBJECT_0, WAIT_FAILED};

        match kernel32::WaitForSingleObject(SEMAPHORE, INFINITE) {
            WAIT_OBJECT_0 => Ok(()),
            WAIT_FAILED => Err(Error::System(io::Error::last_os_error())),
            ret => Err(Error::System(io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "WaitForSingleObject(), unexpected return value \"{:x}\"",
                    ret
                ),
            ))),
        }
    }
}

/// Register signal handler for Ctrl-C.
///
/// Starts a new dedicated signal handling thread. Should only be called once,
/// typically at the start of your program.
///
/// # Example
/// ```no_run
/// ctrlc::set_handler(|| println!("Hello world!")).expect("Error setting Ctrl-C handler");
/// ```
///
/// # Warning
/// On Unix, any existing `SIGINT`, `SIGTERM`(if termination feature is enabled) or `SA_SIGINFO`
/// posix signal handlers will be overwritten. On Windows, multiple handler routines are allowed,
/// but they are called on a last-registered, first-called basis until the signal is handled.
///
/// On Unix, signal dispositions and signal handlers are inherited by child processes created via
/// `fork(2)` on, but not by child processes created via `execve(2)`.
/// Signal handlers are not inherited on Windows.
///
/// # Errors
/// Will return an error if another `ctrlc::set_handler()` handler exists or if a
/// system error occurred while setting the handler.
///
/// # Panics
/// Any panic in the handler will not be caught and will cause the signal handler thread to stop.
///
pub fn set_handler<F>(user_handler: F) -> Result<(), Error>
where
    F: Fn() -> () + 'static + Send,
{
    if INIT.compare_and_swap(false, true, Ordering::SeqCst) {
        return Err(Error::MultipleHandlers);
    }

    unsafe {
        match platform::init_os_handler() {
            Ok(_) => {}
            err => {
                INIT.store(false, Ordering::SeqCst);
                return err;
            }
        }
    }

    thread::spawn(move || loop {
        unsafe {
            platform::block_ctrl_c().expect("Critical system error while waiting for Ctrl-C");
        }
        user_handler();
    });

    Ok(())
}
