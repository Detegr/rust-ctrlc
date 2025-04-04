// Copyright (c) 2017 CtrlC developers
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
//! # #[allow(clippy::needless_doctest_main)]
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
//! # Handling SIGTERM and SIGHUP
//! Handling of `SIGTERM and SIGHUP` can be enabled with `termination` feature. If this is enabled,
//! the handler specified by `set_handler()` will be executed for `SIGINT`, `SIGTERM` and `SIGHUP`.
//!

#[macro_use]

mod error;
mod platform;
use block_outcome::BlockOutcome;
pub use platform::Signal;
mod signal;
pub use signal::*;
mod block_outcome;

pub use error::Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::thread::{self, JoinHandle};

static INIT: AtomicBool = AtomicBool::new(false);
static INIT_LOCK: Mutex<()> = Mutex::new(());

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
/// On Unix, the handler registration for `SIGINT`, (`SIGTERM` and `SIGHUP` if termination feature
/// is enabled) or `SA_SIGINFO` posix signal handlers will be overwritten. On Windows, multiple
/// handler routines are allowed, but they are called on a last-registered, first-called basis
/// until the signal is handled.
///
/// ctrlc::try_set_handler will error (on Unix) if another signal handler exists for the same
/// signal(s) that ctrlc is trying to attach the handler to.
///
/// On Unix, signal dispositions and signal handlers are inherited by child processes created via
/// `fork(2)` on, but not by child processes created via `execve(2)`.
/// Signal handlers are not inherited on Windows.
///
/// # Errors
/// Will return an error if a system error occurred while setting the handler.
///
/// # Panics
/// Any panic in the handler will not be caught and will cause the signal handler thread to stop.
pub fn set_handler<F>(user_handler: F) -> Result<JoinHandle<()>, Error>
where
    F: FnMut() + 'static + Send,
{
    init_and_set_handler(user_handler, true)
}

/// The same as ctrlc::set_handler but errors if a handler already exists for the signal(s).
///
/// # Errors
/// Will return an error if another handler exists or if a system error occurred while setting the
/// handler.
pub fn try_set_handler<F>(user_handler: F) -> Result<JoinHandle<()>, Error>
where
    F: FnMut() + 'static + Send,
{
    init_and_set_handler(user_handler, false)
}

fn init_and_set_handler<F>(user_handler: F, overwrite: bool) -> Result<JoinHandle<()>, Error>
where
    F: FnMut() + 'static + Send,
{
    if !INIT.load(Ordering::Acquire) {
        let _guard = INIT_LOCK.lock().unwrap();

        if !INIT.load(Ordering::Relaxed) {
            let result = set_handler_inner(user_handler, overwrite)?;
            INIT.store(true, Ordering::Release);
            return Ok(result);
        }
    }

    Err(Error::MultipleHandlers)
}

fn set_handler_inner<F>(mut user_handler: F, overwrite: bool) -> Result<JoinHandle<()>, Error>
where
    F: FnMut() + 'static + Send,
{
    unsafe {
        platform::init_os_handler(overwrite)?;
    }

    thread::Builder::new()
        .name("ctrl-c".into())
        .spawn(move || loop {
            unsafe {
                match platform::block_ctrl_c() {
                    Ok(BlockOutcome::Awaited) => {},
                    Ok(BlockOutcome::HandlerRemoved) => break,
                    Err(err) => panic!("Critical system error while waiting for Ctrl-C: {err:?}")
                };
            }
            user_handler();
        })
        .map_err(Error::System)
}


/// Same as [`ctrlc::set_handler`], but uses [`std::ops::FnOnce`] as a handler that only handles one interrupt.
/// 
/// Register signal handler for Ctrl-C.
///
/// Starts a new dedicated signal handling thread. Should only be called at the start of the program, or after 
/// last `_once`-handler already fired (for example, via `.join()`).
///
/// # Example
/// ```no_run
/// 
/// # use ctrlc::*;
/// 
/// let fires = 0;
/// let handle = ctrlc::set_handler_once(move || fires + 1).unwrap();
/// 
/// // interrupt_and_wait(); // platform-dependant
/// 
/// // First unwrap for thread join, second one to `Option` because handler can be removed without firing.
/// let fires = handle.join().unwrap().unwrap(); 
/// assert_eq!(fires, 1);
/// 
/// assert!(ctrlc::remove_all_handlers().is_err()); // This handler should be already removed after firing once.
/// ```
pub fn set_handler_once<F, T>(user_handler: F) -> Result<JoinHandle<Option<F::Output>>, Error>
where
    F: FnOnce() -> T + 'static + Send,
    T: 'static + Send
{
    init_and_set_handler_once(user_handler, true)
}

/// The same as [`ctrlc::try_set_handler`] but uses [`std::ops::FnOnce`] as a handler that only handles one interrupt.
/// The same as [`ctrlc::set_handler_once`] but errors if a handler already exists for the signal(s).
///
/// # Errors
/// Will return an error if another handler exists or if a system error occurred while setting the
/// handler.
pub fn try_set_handler_once<F, T>(user_handler: F) -> Result<JoinHandle<Option<F::Output>>, Error>
where
    F: FnOnce() -> T + 'static + Send,
    T: 'static + Send
{
    init_and_set_handler_once(user_handler, false)
}


fn init_and_set_handler_once<F, T>(user_handler: F, overwrite: bool) -> Result<JoinHandle<Option<F::Output>>, Error>
where
    F: FnOnce() -> T + 'static + Send,
    T: 'static + Send
{
    if !INIT.load(Ordering::Acquire) {
        let _guard = INIT_LOCK.lock().unwrap();

        if !INIT.load(Ordering::Relaxed) {
            let handle = set_handler_inner_once(user_handler, overwrite)?;
            INIT.store(true, Ordering::Release);
            return Ok(handle);
        }
    }

    Err(Error::MultipleHandlers)
}


fn set_handler_inner_once<F, T>(user_handler: F, overwrite: bool) -> Result<JoinHandle<Option<F::Output>>, Error>
where
    F: FnOnce() -> T + 'static + Send,
    T: 'static + Send,
{
    unsafe {
        platform::init_os_handler(overwrite)?;
    }

    let thread = thread::Builder::new()
        .name("ctrl-c".into())
        .spawn(move || {
            let outcome = unsafe {
                platform::block_ctrl_c().expect("Critical system error while waiting for Ctrl-C")
            };
            if outcome == BlockOutcome::HandlerRemoved {
                return None;
            }
            let result = user_handler();

            match remove_all_handlers() {
                Ok(()) |
                Err(Error::HandlerRemoved) => {},
                _ => eprintln!("[ctrlc] System error after waiting for Ctrl-C"),
            };
            Some(result)
        })
        .map_err(Error::System)?;

    Ok(thread)
}

/// Removes all previously added handlers
pub fn remove_all_handlers() -> Result<(), Error> {
    if !INIT.load(Ordering::Acquire) {
        return Err(Error::HandlerRemoved);
    }
    unsafe {
        platform::deinit_os_handler()?;
        INIT.store(false, Ordering::Relaxed);
    }
    Ok(())
}
