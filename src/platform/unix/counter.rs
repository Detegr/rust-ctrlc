// Copyright (c) 2017 CtrlC developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

use error::Error;
use platform::unix::nix::sys::signal::Signal;
use platform::unix::nix;
use self::nix::sys::signal as nix_signal;
use signal::SignalType;
use std::sync::atomic::AtomicUsize;

struct SignalMap {
    signals: Box<[usize]>,
    counters: Box<[AtomicUsize]>,
}
impl SignalMap {
    fn get_counter(&self, signal: nix::libc::c_int) -> Option<&AtomicUsize> {
        self.signals
            .iter()
            .zip(self.counters.iter())
            .find(|&(sig, _)| *sig == signal as usize)
            .map(|sigmap| sigmap.1)
    }
}

lazy_static! {
    static ref SIGNALS: SignalMap = {
        let signals = Signal::iterator().map(|sig| sig as usize).collect::<Vec<_>>();
        let counters = signals.clone().into_iter().map(|_| AtomicUsize::new(0)).collect::<Vec<_>>();
        SignalMap {
            signals: signals.into_boxed_slice(),
            counters: counters.into_boxed_slice(),
        }
    };
}

/// Sets up a signal handler and counts the amount of times the signal handler has been run.
pub struct Counter {
    signal: nix_signal::Signal,
}

impl Counter {
    extern "C" fn os_handler(signum: nix::libc::c_int) {
        use std::sync::atomic::Ordering;
        if let Some(counter) = SIGNALS.get_counter(signum) {
            counter.fetch_add(1, Ordering::AcqRel);
        }
    }
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
    ///     while counter.get().unwrap() == 0 {
    ///         thread::sleep(time::Duration::from_millis(10));
    ///     }
    ///     println!("Got it! Exiting...");
    /// }
    pub fn new(signal: SignalType) -> Result<Counter, Error> {
        let raw_signal = match signal {
            SignalType::Ctrlc => Signal::SIGINT,
            SignalType::Termination => Signal::SIGTERM,
            SignalType::Other(ref s) => *s,
        };

        if !SIGNALS.signals.iter().any(|&s| raw_signal as usize == s) {
            return Err(Error::NoSuchSignal(signal));
        }

        let handler = nix_signal::SigHandler::Handler(Counter::os_handler);
        let new_action = nix_signal::SigAction::new(
            handler,
            nix_signal::SA_RESTART,
            nix_signal::SigSet::empty(),
        );
        let _old = unsafe { nix_signal::sigaction(raw_signal, &new_action)? };
        Ok(Counter { signal: raw_signal })
    }
    /// Gets the value of the counter using an atomic operation.
    ///
    /// # Note
    /// The value returned may not be the value of the counter anymore.
    /// This function accesses the counter atomically, but loads the value into normal `usize`
    /// variable, so the counter may or may not have changed during the time this function returns
    ///
    /// # Errors
    /// Returns `None` if the signal specified in `SignalType::Other` is not available in the
    /// system.
    pub fn get(&self) -> Option<usize> {
        use std::sync::atomic::Ordering;
        SIGNALS
            .get_counter(self.signal as nix::libc::c_int)
            .and_then(|counter| Some(counter.load(Ordering::Acquire)))
    }
}

impl Drop for Counter {
    /// Dropping the counter unregisters the signal handler attached to the counter.
    fn drop(&mut self) {
        let new_action = nix_signal::SigAction::new(
            nix_signal::SigHandler::SigDfl,
            nix_signal::SaFlags::empty(),
            nix_signal::SigSet::empty(),
        );
        let _old = unsafe { nix_signal::sigaction(self.signal, &new_action) };
    }
}
