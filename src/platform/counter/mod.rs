// Copyright (c) 2021 CtrlC developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

use crate::error::Error;
use crate::signal::SignalType;
use crate::signalmap::SIGMAP;

#[cfg(unix)]
mod unix;
#[cfg(unix)]
use self::unix::*;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use self::windows::*;

/// Counter abstraction for signals
pub struct Counter {
    signal: crate::platform::Signal,
}
impl Counter {
    /// Creates a new counter. [SignalType](enum.SignalType.html) defines the signal you want the
    /// counter to count.
    ///
    /// # Example
    ///
    /// ```no_run
    /// extern crate ctrlc;
    /// use std::thread;
    /// use std::time;
    ///
    /// fn main() {
    ///     let counter = ctrlc::Counter::new(ctrlc::SignalType::Ctrlc).unwrap();
    ///     println!("Waiting for Ctrl-C...");
    ///     while counter.get() == 0 {
    ///         thread::sleep(time::Duration::from_millis(10));
    ///     }
    ///     println!("Got it! Exiting...");
    /// }
    /// ```
    ///
    /// # Errors
    /// Errors if the signal specified in `SignalType::Other` is not available in the system.
    ///
    /// Errors if there already exists a Counter for the signal.
    ///
    /// On *nix systems an error is returned if the system already has a non-default signal handler for
    /// the registered signal.
    pub fn new(signal: SignalType) -> Result<Counter, Error> {
        let platform_signal = signal.into();

        if !SIGMAP.signals.iter().any(|&s| platform_signal == s) {
            return Err(Error::NoSuchSignal(platform_signal));
        }

        set_handler(platform_signal)?;

        Ok(Counter {
            signal: platform_signal,
        })
    }

    /// Gets the value of the counter using an atomic operation.
    ///
    /// # Note
    /// The value returned may not be the value of the counter anymore.
    /// This function accesses the counter atomically, but loads the value into normal `usize`
    /// variable, so the counter may or may not have changed during the time this function returns.
    pub fn get(&self) -> usize {
        use std::sync::atomic::Ordering;
        SIGMAP
            .get_counter(&self.signal)
            .unwrap()
            .load(Ordering::Acquire)
    }
}
impl Drop for Counter {
    /// Dropping the counter unregisters the signal handler attached to the counter.
    fn drop(&mut self) {
        reset_handler(self.signal);
    }
}
