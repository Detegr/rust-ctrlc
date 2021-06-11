// Copyright (c) 2017 CtrlC developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

#![warn(missing_docs)]

//! Cross platform signal handling library
//!
//! Provides two different APIs for signal handling:
//!  * [Counter](struct.Counter.html)
//!  * [Channel](struct.Channel.html)
//!
// [set_handler()](fn.set_handler.html) allows setting a handler closure which is executed on
// `Ctrl+C`. On Unix, this corresponds to a `SIGINT` signal. On windows, `Ctrl+C` corresponds to
// [`CTRL_C_EVENT`][HandlerRoutine] or [`CTRL_BREAK_EVENT`][HandlerRoutine].
//
//! # Counter example
//! ```no_run
//! extern crate ctrlc;
//! use std::thread;
//! use std::time;
//!
//! fn main() {
//!     let counter = ctrlc::Counter::new(ctrlc::SignalType::Ctrlc).unwrap();
//!     println!("Waiting for Ctrl-C...");
//!     while counter.get() == 0 {
//!         thread::sleep(time::Duration::from_millis(10));
//!     }
//!     println!("Got it! Exiting...");
//! }
//! ```
//! # Channel example
//!
//! ```no_run
//! extern crate ctrlc;
//! use std::thread;
//! use std::time;
//!
//! fn main() {
//!     let channel = ctrlc::Channel::new(ctrlc::SignalType::Ctrlc).unwrap();
//!     println!("Waiting for Ctrl-C...");
//!     channel.recv().unwrap();
//!     println!("Got it! Exiting...");
//! }
//! ```

extern crate byteorder;
#[macro_use]
extern crate lazy_static;

mod channel;
mod counter;
mod error;
mod platform;
mod signal;
mod signalevent;
mod signalmap;

pub use channel::*;
pub use counter::*;
pub use platform::Signal;
pub use signal::*;

pub use error::Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

static INIT: AtomicBool = AtomicBool::new(false);

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
#[deprecated(note = "Use Counter or Channel to prevent creating an extra thread")]
pub fn set_handler<F>(mut user_handler: F) -> Result<(), Error>
where
    F: FnMut() -> () + 'static + Send,
{
    if INIT.compare_exchange(false, true, Ordering::SeqCst, Ordering::Acquire).is_err() {
        return Err(Error::MultipleHandlers);
    }

    unsafe {
        match platform::init_os_handler() {
            Ok(_) => {}
            Err(err) => {
                INIT.store(false, Ordering::SeqCst);
                return Err(err.into());
            }
        }
    }

    thread::Builder::new()
        .name("ctrl-c".into())
        .spawn(move || loop {
            unsafe {
                platform::block_ctrl_c().expect("Critical system error while waiting for Ctrl-C");
            }
            user_handler();
        })
        .expect("failed to spawn thread");

    Ok(())
}
